//! Per-mark artistic blending for an isolated chart. The backdrop contains
//! the Scene background and earlier draws in paint order. A mark's fill and
//! stroke form one transparent source. Its opacity and clips apply to that
//! source once, before it composites over the backdrop.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint, PathCommand};

use crate::scene::ensure;
use crate::{Geometry, Mark, Result, Scene};

/// The CSS artistic modes also exposed by Nivo. Normal uses source-over.
/// Destination-replacing Porter–Duff operations are outside this profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    pub const fn css_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
            Self::ColorDodge => "color-dodge",
            Self::ColorBurn => "color-burn",
            Self::HardLight => "hard-light",
            Self::SoftLight => "soft-light",
            Self::Difference => "difference",
            Self::Exclusion => "exclusion",
            Self::Hue => "hue",
            Self::Saturation => "saturation",
            Self::Color => "color",
            Self::Luminosity => "luminosity",
        }
    }

    pub(crate) const fn canvas_name(self) -> &'static str {
        match self {
            Self::Normal => "source-over",
            _ => self.css_name(),
        }
    }
}

/// A checked mode map keyed by stable Scene mark ID. The empty profile keeps
/// the ordinary rendering path and can be reused across scenes. Normal
/// entries are checked then omitted from storage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkBlends {
    modes: BTreeMap<String, BlendMode>,
}

impl MarkBlends {
    /// Internal paint identities are already checked by the motion solver's
    /// scene admission. The transient union still has its own byte budget.
    pub(crate) fn from_motion_entries(
        entries: impl IntoIterator<Item = (String, BlendMode)>,
    ) -> Result<Self> {
        let mut modes = BTreeMap::new();
        let mut bytes = 0usize;
        for (id, mode) in entries {
            if mode == BlendMode::Normal {
                continue;
            }
            bytes += id.len();
            ensure(
                bytes <= 4_000_000,
                "motion blend mark ID byte budget exceeded",
            )?;
            ensure(
                modes.insert(id, mode).is_none(),
                "duplicate motion blend paint identity",
            )?;
        }
        Ok(Self { modes })
    }

    pub fn new<I, S>(scene: &Scene, modes: I) -> Result<Self>
    where
        I: IntoIterator<Item = (S, BlendMode)>,
        S: Into<String>,
    {
        scene.validate()?;
        let known: BTreeSet<_> = scene.marks.iter().map(|mark| mark.id.as_str()).collect();
        let mut ids = BTreeSet::new();
        let mut map = BTreeMap::new();
        let mut bytes = 0usize;
        for (id, mode) in modes {
            let id = id.into();
            ensure(
                known.contains(id.as_str()),
                "blend mode references an unknown mark",
            )?;
            ensure(ids.insert(id.clone()), "duplicate blend mode for mark")?;
            bytes += id.len();
            ensure(bytes <= 4_000_000, "blend mark ID byte budget exceeded")?;
            if mode != BlendMode::Normal {
                map.insert(id, mode);
            }
        }
        Ok(Self { modes: map })
    }

    pub fn mode(&self, mark_id: &str) -> BlendMode {
        self.modes.get(mark_id).copied().unwrap_or_default()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, BlendMode)> + '_ {
        self.modes.iter().map(|(id, mode)| (id.as_str(), *mode))
    }

    pub fn is_empty(&self) -> bool {
        self.modes.is_empty()
    }

    /// Check that all explicit non-normal modes can be applied to this Scene.
    pub fn validate(&self, scene: &Scene) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        let known: BTreeSet<_> = scene.marks.iter().map(|mark| mark.id.as_str()).collect();
        ensure(
            self.modes.keys().all(|id| known.contains(id.as_str())),
            "blend mode references an unknown mark",
        )
    }

    /// Project this profile onto a filtered scene, for example after hiding a
    /// series. Any newly added mark defaults to Normal.
    pub fn for_scene(&self, scene: &Scene) -> Self {
        if self.is_empty() {
            return Self::default();
        }
        Self {
            modes: scene
                .marks
                .iter()
                .filter_map(|mark| {
                    self.modes
                        .get(&mark.id)
                        .map(|mode| (mark.id.clone(), *mode))
                })
                .collect(),
        }
    }
}

/// Whether a mark needs one isolated source. Paint servers and general paths
/// conservatively count as potential fill; normalized point strokes and
/// zero-area primitives remain single-paint fast paths.
pub(crate) fn requires_isolation(mark: &Mark, mode: BlendMode) -> bool {
    if mode != BlendMode::Normal {
        return true;
    }
    if !(mark.opacity > 0. && mark.opacity < 1.)
        || !(mark.fill_paint.is_some() || mark.fill.is_some_and(|c| c.3 != 0))
        || !mark.stroke.is_some_and(|s| s.width > 0. && s.color.3 != 0)
    {
        return false;
    }
    match &mark.geometry {
        Geometry::Rect { width, height, .. } => *width > 0. && *height > 0.,
        Geometry::Circle { radius, .. } => *radius > 0.,
        Geometry::Path(commands) => {
            crate::render::point_stroke_position(&mark.geometry).is_none()
                && commands
                    .iter()
                    .any(|c| matches!(c, PathCommand::Line(_) | PathCommand::Cubic { .. }))
        }
    }
}

pub(crate) fn has_isolated_marks(scene: &Scene, modes: &MarkBlends) -> bool {
    scene
        .opacity_groups
        .iter()
        .any(|g| g.opacity > 0. && g.opacity < 1.)
        || scene
            .marks
            .iter()
            .any(|mark| requires_isolation(mark, modes.mode(&mark.id)))
}

pub(crate) fn validate_opacity_group_layers(scene: &Scene, modes: &MarkBlends) -> Result<()> {
    if scene.opacity_groups.is_empty() {
        return Ok(());
    }
    let marks: BTreeMap<_, _> = scene.marks.iter().map(|m| (m.id.as_str(), m)).collect();
    for group in scene
        .opacity_groups
        .iter()
        .filter(|g| g.opacity > 0. && g.opacity < 1.)
    {
        ensure(
            group.members.iter().all(|id| {
                marks
                    .get(id.as_str())
                    .is_some_and(|m| !requires_isolation(m, modes.mode(id)))
            }),
            "Canvas opacity group cannot contain an isolated mark blend layer",
        )?;
    }
    Ok(())
}

pub(crate) fn group_blend_bounds(scene: &Scene) -> Vec<Option<BlendBounds>> {
    if scene.opacity_groups.is_empty() {
        return Vec::new();
    }
    let marks: BTreeMap<_, _> = scene.marks.iter().map(|m| (m.id.as_str(), m)).collect();
    scene
        .opacity_groups
        .iter()
        .map(|group| {
            if !(group.opacity > 0. && group.opacity < 1.) {
                return None;
            }
            group
                .members
                .iter()
                .filter_map(|id| marks.get(id.as_str()))
                .filter_map(|m| mark_blend_bounds(m))
                .reduce(|a, b| BlendBounds {
                    min: [a.min[0].min(b.min[0]), a.min[1].min(b.min[1])],
                    max: [a.max[0].max(b.max[0]), a.max[1].max(b.max[1])],
                })
        })
        .collect()
}

/// Per-frame reusable offscreen dimensions and total visited physical area.
/// Shared limits for artistic blending and Normal multi-paint opacity groups.
pub(crate) const MAX_BLEND_SIDE: u32 = 8_192;
pub(crate) const MAX_BLEND_SCRATCH_PIXELS: u64 = 16_777_216;
pub(crate) const MAX_BLEND_FRAME_PIXELS: u64 = 67_108_864;

#[derive(Clone, Copy, Debug)]
pub(crate) struct BlendBounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

impl BlendBounds {
    fn include(&mut self, point: GraphicPoint) {
        self.min = [self.min[0].min(point.x), self.min[1].min(point.y)];
        self.max = [self.max[0].max(point.x), self.max[1].max(point.y)];
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let result = Self {
            min: [self.min[0].max(other.min[0]), self.min[1].max(other.min[1])],
            max: [self.max[0].min(other.max[0]), self.max[1].min(other.max[1])],
        };
        (result.min[0] < result.max[0] && result.min[1] < result.max[1]).then_some(result)
    }
}

fn geometry_bounds(geometry: &Geometry) -> Option<BlendBounds> {
    Some(match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => BlendBounds {
            min: [*x, *y],
            max: [x + width, y + height],
        },
        Geometry::Circle { x, y, radius } => BlendBounds {
            min: [x - radius, y - radius],
            max: [x + radius, y + radius],
        },
        Geometry::Path(commands) => {
            let mut bounds = BlendBounds {
                min: [f64::INFINITY; 2],
                max: [f64::NEG_INFINITY; 2],
            };
            for command in commands {
                match command {
                    PathCommand::Move(p) | PathCommand::Line(p) => bounds.include(*p),
                    PathCommand::Cubic { first, second, end } => {
                        bounds.include(*first);
                        bounds.include(*second);
                        bounds.include(*end);
                    }
                    PathCommand::Close => {}
                }
            }
            if bounds.min[0] == f64::INFINITY {
                return None;
            }
            bounds
        }
    })
}

pub(crate) fn mark_blend_bounds(mark: &Mark) -> Option<BlendBounds> {
    let stroke = mark.stroke.filter(|stroke| stroke.color.3 != 0);
    let fill = mark.fill_paint.is_some() || mark.fill.is_some_and(|color| color.3 != 0);
    if mark.opacity == 0. || !fill && stroke.is_none() {
        return None;
    }
    let mut bounds = geometry_bounds(&mark.geometry)?;
    // Both direct Canvas and CPU/SVG use miter limit four. Include that
    // maximum join as well as all endpoint caps and curved path supports.
    if let Some(stroke) = stroke {
        bounds.min = bounds.min.map(|v| v - stroke.width * 2.);
        bounds.max = bounds.max.map(|v| v + stroke.width * 2.);
    }
    if let Some(rect) = mark.clip {
        bounds = bounds.intersect(BlendBounds {
            min: [rect.x, rect.y],
            max: [rect.x + rect.width, rect.y + rect.height],
        })?;
    }
    if let Some(path) = &mark.clip_path {
        bounds = bounds.intersect(geometry_bounds(&path.geometry)?)?;
    }
    (bounds.min[0] < bounds.max[0] && bounds.min[1] < bounds.max[1]).then_some(bounds)
}

#[cfg(any(feature = "raster", all(feature = "canvas", target_arch = "wasm32")))]
mod physical {
    use super::*;
    use crate::{ChartError, ZoomTransform};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct BlendRegion {
        pub x: u32,
        pub y: u32,
        pub width: u32,
        pub height: u32,
    }

    impl BlendRegion {
        pub(crate) fn pixels(self) -> u64 {
            u64::from(self.width) * u64::from(self.height)
        }
    }

    pub(crate) struct BlendPlan {
        pub(crate) regions: Vec<Option<BlendRegion>>,
        pub(crate) scratch: [u32; 2],
        #[cfg(feature = "raster")]
        pub(crate) mark_count: usize,
    }

    impl BlendPlan {
        pub(crate) fn new(
            scene: &Scene,
            modes: &MarkBlends,
            size: [u32; 2],
            dpr: f64,
            view: ZoomTransform,
        ) -> Result<Self> {
            validate_opacity_group_layers(scene, modes)?;
            let mut regions = Vec::new();
            let mut scratch = [0, 0];
            let mut work = 0u64;
            if has_isolated_marks(scene, modes) {
                regions.reserve(scene.marks.len() + scene.opacity_groups.len());
                let bounds = scene
                    .marks
                    .iter()
                    .map(|mark| {
                        requires_isolation(mark, modes.mode(&mark.id))
                            .then(|| mark_blend_bounds(mark))
                            .flatten()
                    })
                    .chain(group_blend_bounds(scene));
                for bounds in bounds {
                    let region = bounds.and_then(|b| physical_bounds(b, size, dpr, view));
                    if let Some(region) = region {
                        scratch[0] = scratch[0].max(region.width);
                        scratch[1] = scratch[1].max(region.height);
                        work = work
                            .checked_add(region.pixels())
                            .ok_or_else(|| ChartError("blend pixel work overflow".into()))?;
                        ensure(
                            work <= MAX_BLEND_FRAME_PIXELS,
                            "blend frame pixel work budget exceeded",
                        )?;
                        ensure(
                            scratch.into_iter().all(|side| side <= MAX_BLEND_SIDE)
                                && u64::from(scratch[0]) * u64::from(scratch[1])
                                    <= MAX_BLEND_SCRATCH_PIXELS,
                            "blend scratch pixel budget exceeded",
                        )?;
                    }
                    regions.push(region);
                }
            }
            Ok(Self {
                regions,
                scratch,
                #[cfg(feature = "raster")]
                mark_count: scene.marks.len(),
            })
        }
    }

    fn physical_bounds(
        bounds: BlendBounds,
        size: [u32; 2],
        dpr: f64,
        view: ZoomTransform,
    ) -> Option<BlendRegion> {
        let shifts = [view.translate_x, view.translate_y];
        let project = |v, i: usize| (v * view.scale + shifts[i]) * dpr;
        let mut start = [0; 2];
        let mut end = [0; 2];
        for i in 0..2 {
            let lo: f64 = project(bounds.min[i], i);
            let hi: f64 = project(bounds.max[i], i);
            if lo >= f64::from(size[i]) || hi <= 0. {
                return None;
            }
            start[i] = (lo.floor() - 2.).clamp(0., f64::from(size[i])) as u32;
            end[i] = (hi.ceil() + 2.).clamp(0., f64::from(size[i])) as u32;
        }
        (start[0] < end[0] && start[1] < end[1]).then_some(BlendRegion {
            x: start[0],
            y: start[1],
            width: end[0] - start[0],
            height: end[1] - start[1],
        })
    }
}

#[cfg(any(feature = "raster", all(feature = "canvas", target_arch = "wasm32")))]
pub(crate) use physical::{BlendPlan, BlendRegion};
