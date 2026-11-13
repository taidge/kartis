//! Point overlays share the fitted polygon projection (Nivo geo/hooks.js).
//! Area encoding interpolates squared radii, i.e. circle area, rather than
//! radius. See local D3 docs/d3-shape/symbol.md and d3-scale/pow.md. Discrete
//! numeric colors use uniform, left-closed bins (D3 d3-scale/quantize.md).
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicStroke, StrokeCap};

use super::{GeoMapLayout, GeoProjectionContext, valid_position};
use crate::scene::ensure;
use crate::{
    Color, DrawItem, Geometry, Label, LegendItem, MAX_MARKS, Mark, Result, Scene, TextAnchor,
    Theme, ValueFormat,
};

#[path = "geo/symbol_clip.rs"]
mod disk_clip;

pub const MAX_GEO_SYMBOLS: usize = 10_000;
pub const MAX_GEO_SYMBOL_LEGEND_VALUES: usize = 32;

/// Symbols have equal area for the same magnitude and equivalent-circle
/// radius: area = pi*r². Radius encoding scales this equivalent radius.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoSymbolShape {
    #[default]
    Circle,
    Square,
    Diamond,
    Triangle,
    Cross,
}
impl GeoSymbolShape {
    fn vertices(self, r: f64) -> Vec<[f64; 2]> {
        let area = std::f64::consts::PI * r * r;
        match self {
            Self::Circle => vec![],
            Self::Square => {
                let h = area.sqrt() / 2.;
                vec![[-h, -h], [h, -h], [h, h], [-h, h]]
            }
            Self::Diamond => {
                let x = (area / (2. * 3_f64.sqrt())).sqrt();
                let y = x * 3_f64.sqrt();
                vec![[0., -y], [x, 0.], [0., y], [-x, 0.]]
            }
            Self::Triangle => {
                let x = (area / 3_f64.sqrt()).sqrt();
                let h = x * 3_f64.sqrt();
                vec![[0., -2. * h / 3.], [x, h / 3.], [-x, h / 3.]]
            }
            Self::Cross => {
                let a = (area / 20.).sqrt();
                [
                    [-3., -1.],
                    [-1., -1.],
                    [-1., -3.],
                    [1., -3.],
                    [1., -1.],
                    [3., -1.],
                    [3., 1.],
                    [1., 1.],
                    [1., 3.],
                    [-1., 3.],
                    [-1., 1.],
                    [-3., 1.],
                ]
                .map(|[x, y]| [x * a, y * a])
                .to_vec()
            }
        }
    }
    fn bounds(self, r: f64) -> [f64; 4] {
        if self == Self::Circle {
            return [-r, -r, r, r];
        }
        self.vertices(r).into_iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| {
                [
                    b[0].min(p[0]),
                    b[1].min(p[1]),
                    b[2].max(p[0]),
                    b[3].max(p[1]),
                ]
            },
        )
    }
    fn geometry(self, x: f64, y: f64, r: f64) -> Geometry {
        if self == Self::Circle {
            return Geometry::Circle { x, y, radius: r };
        }
        let vertices = self.vertices(r);
        let mut path = vertices
            .into_iter()
            .enumerate()
            .map(|(i, [a, b])| {
                let p = glory::render::GraphicPoint::new(x + a, y + b);
                if i == 0 {
                    glory::render::PathCommand::Move(p)
                } else {
                    glory::render::PathCommand::Line(p)
                }
            })
            .collect::<Vec<_>>();
        path.push(glory::render::PathCommand::Close);
        Geometry::Path(path)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoSymbol {
    pub id: String,
    pub series: String,
    pub label: String,
    /// Longitude/latitude in degrees, within ±180/±90.
    pub position: [f64; 2],
    /// Finite nonnegative magnitude, or missing. Zero is distinct from missing.
    pub value: Option<f64>,
}
impl GeoSymbol {
    pub fn new(id: impl Into<String>, position: [f64; 2], value: Option<f64>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            series: "symbols".into(),
            position,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoSymbolSize {
    /// Linear interpolation of squared radii; area is proportional to value
    /// with the default zero-based domain and radius range.
    #[default]
    Area,
    Radius,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub enum GeoMissingSymbol {
    #[default]
    Skip,
    Draw {
        radius: f64,
        color: Color,
        label: String,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum GeoSymbolColor {
    /// Lexical series order gives deterministic palette assignment.
    Series(Vec<Color>),
    Fixed(Color),
    /// Quantized, clamped equal-width bins. None uses the size domain.
    Value {
        domain: Option<[f64; 2]>,
        colors: Vec<Color>,
    },
}
impl Default for GeoSymbolColor {
    fn default() -> Self {
        Self::Series(Theme::default().colors)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoLegendOptions {
    /// Top left in logical Scene pixels, independent of the map transform.
    pub position: [f64; 2],
    pub title: String,
    /// Minimum row height; increased to fit each actual circle/swatch.
    pub row_height: f64,
    pub text_color: Color,
    pub text_size: f64,
    pub format: ValueFormat,
}
impl Default for GeoLegendOptions {
    fn default() -> Self {
        Self {
            position: [20., 20.],
            title: String::new(),
            row_height: 24.,
            text_color: Theme::default().text,
            text_size: 12.,
            format: ValueFormat::Auto,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoSizeLegend {
    pub layout: GeoLegendOptions,
    /// Exact representative magnitudes, rendered with the same radius mapping.
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoSymbolOptions {
    pub shape: GeoSymbolShape,
    pub size: GeoSymbolSize,
    /// Default [0, maximum present magnitude], or [0, 1] for all zero/missing.
    /// Explicit domains must be finite, nonnegative and strictly increasing.
    pub domain: Option<[f64; 2]>,
    /// Logical radii, ascending in 0..=4096. Values outside domain clamp.
    pub radius_range: [f64; 2],
    /// Zero bypasses radius_range; zero radius omits a mark (no phantom hit).
    pub zero_radius: f64,
    pub missing: GeoMissingSymbol,
    pub color: GeoSymbolColor,
    pub stroke: Option<GraphicStroke>,
    pub labels: bool,
    pub label_color: Color,
    pub label_size: f64,
    pub value_format: ValueFormat,
    /// Clips visible circles to the same plot rectangle as the fitted map.
    /// Hemisphere-projection back-face culling always applies, even when false.
    /// This is rectangular clipping; horizon circles may extend beyond the
    /// globe silhouette. Shapes use their actual bounds for admission and label
    /// offsets. Text labels are independent of this rectangular clipping.
    pub clip: bool,
    /// Clip symbol fills and strokes to the fitted azimuthal projection disk.
    /// With a stroke, the original geometry is retained and a Scene clip path
    /// masks the entire painted shape; no artificial horizon stroke is added.
    /// Fill-only symbols retain the bounded geometric intersection profile.
    /// Conic families instead use their checked finite world footprint, masking
    /// the full original ink without a fake circular horizon or cut-edge stroke.
    /// Other unsupported projections reject this option. Disk radii must be
    /// 0.01..=1,000,000 logical pixels and centers
    /// within Scene's coordinate bounds. Circular boundaries use small cubic
    /// arcs with radial error below 0.00003 logical pixels in this profile.
    /// Size/color legends remain complete. Text clipping is independently opt-in.
    pub clip_to_projection: bool,
    /// Mask only newly appended location labels to the projection disk. This
    /// supports the same azimuthal disk or conic finite footprint. Legend labels
    /// remain complete.
    pub clip_labels_to_projection: bool,
    pub size_legend: Option<GeoSizeLegend>,
    /// Actual categorical swatches or numeric interval swatches, with labels.
    /// A drawn missing-value style adds a distinct missing row.
    pub color_legend: Option<GeoLegendOptions>,
}
impl Default for GeoSymbolOptions {
    fn default() -> Self {
        Self {
            shape: GeoSymbolShape::Circle,
            size: GeoSymbolSize::Area,
            domain: None,
            radius_range: [0., 24.],
            zero_radius: 0.,
            missing: GeoMissingSymbol::Skip,
            color: GeoSymbolColor::default(),
            stroke: Some(GraphicStroke {
                color: Color(255, 255, 255, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            labels: false,
            label_color: Theme::default().text,
            label_size: 12.,
            value_format: ValueFormat::Auto,
            clip: true,
            clip_to_projection: false,
            clip_labels_to_projection: false,
            size_legend: None,
            color_legend: None,
        }
    }
}

/// Additive glyph overlay. Existing `add_symbols` and GeoSymbolShape defaults
/// are unchanged. The nested legacy shape is ignored here; `shape` is explicit.
/// Existing Area/Radius mappings produce equivalent r, then D3 size = pi*r².
/// For stroked variants this is only a nominal parameter, not painted area.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGlyphOptions {
    pub symbols: GeoSymbolOptions,
    pub shape: crate::GlyphShape,
    pub rotation: f64,
    /// Source series hidden before appending ink; full domain/palette/fit and
    /// legend identities remain stable. Existing map marks are never filtered.
    pub hidden: BTreeSet<String>,
}
impl Default for GeoGlyphOptions {
    fn default() -> Self {
        Self {
            symbols: Default::default(),
            shape: Default::default(),
            rotation: 0.,
            hidden: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGlyphSite {
    pub id: String,
    pub source_index: usize,
    pub mark_id: String,
    pub series: String,
    pub label: String,
    pub value: Option<f64>,
    pub formatted_value: String,
    pub color: Color,
    pub glyph: crate::GlyphGeometry,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoGlyphLayer {
    pub sites: Vec<GeoGlyphSite>,
    pub domain: [f64; 2],
    pub color_domain: [f64; 2],
    pub hidden: BTreeSet<String>,
}
impl GeoGlyphLayer {
    /// Bind the authored target Scene, using actual Path endpoints to retain
    /// the D3 origin through animation. Exact picking keeps the real contour.
    pub fn interaction_binding(&self, scene: &Scene) -> Result<crate::InteractionBinding> {
        let mut binding = crate::InteractionBinding::default();
        let lookup = scene
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect::<BTreeMap<_, _>>();
        ensure(
            self.sites.len() <= MAX_GEO_SYMBOLS,
            "geographic glyph metadata budget exceeded",
        )?;
        for site in &self.sites {
            site.glyph.validate()?;
            let mark = lookup
                .get(site.mark_id.as_str())
                .ok_or_else(|| crate::ChartError("geographic glyph mark is absent".into()))?;
            ensure(
                mark.geometry == site.glyph.geometry
                    && mark.series == site.series
                    && mark.value == site.value.unwrap_or(0.),
                "stale geographic glyph metadata",
            )?;
            if !site.glyph.anchor_indices.is_empty() {
                binding.anchors.insert(
                    site.mark_id.clone(),
                    crate::InteractionAnchor::path_points(
                        &mark.geometry,
                        site.glyph.anchor,
                        &site.glyph.anchor_indices,
                    )?,
                );
            }
            binding.sites.push(crate::InteractionSite {
                mark_id: site.mark_id.clone(),
                series: site.series.clone(),
                label: site.label.clone(),
                value: site.value,
                formatted_value: site.value.map(|_| site.formatted_value.clone()),
                color: site.color,
                position: site.glyph.anchor,
            });
        }
        binding.validate(scene)?;
        Ok(binding)
    }
}
fn glyph_at(glyph: &GeoGlyphOptions, center: [f64; 2], r: f64) -> Result<crate::GlyphGeometry> {
    let size = std::f64::consts::PI * r * r;
    ensure(
        r == 0. || size > 0.,
        "positive geographic glyph nominal size underflow",
    )?;
    crate::glyph_geometry(
        center,
        &crate::GlyphOptions {
            shape: glyph.shape,
            size,
            rotation: glyph.rotation,
        },
    )
}
fn extent(o: &GeoSymbolOptions, glyph: Option<&GeoGlyphOptions>, r: f64) -> Result<[f64; 4]> {
    if let Some(g) = glyph {
        let b = glyph_at(g, [0., 0.], r)?.bounds;
        Ok([b.x, b.y, b.x + b.width, b.y + b.height])
    } else {
        Ok(o.shape.bounds(r))
    }
}
fn glyph_ink(
    glyph: &crate::GlyphGeometry,
    id: impl Into<String>,
    color: Color,
    stroke: Option<GraphicStroke>,
) -> Result<Mark> {
    if glyph.paint_mode == crate::GlyphPaintMode::Stroke {
        let stroke = stroke.ok_or_else(|| {
            crate::ChartError("stroked geographic glyph requires a stroke".into())
        })?;
        crate::glyph_mark(id, glyph, None, Some(GraphicStroke { color, ..stroke }))
    } else {
        crate::glyph_mark(id, glyph, Some(color), stroke)
    }
}

fn domain_valid(domain: [f64; 2]) -> Result<()> {
    ensure(
        domain.iter().all(|v| v.is_finite() && *v >= 0.) && domain[1] > domain[0],
        "invalid geographic symbol domain",
    )
}
fn radius_valid(radius: f64) -> bool {
    radius.is_finite() && (0. ..=4096.).contains(&radius)
}
fn legend_valid(layout: &GeoLegendOptions) -> Result<()> {
    ensure(
        layout
            .position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            && layout.title.len() <= 1024
            && layout.row_height.is_finite()
            && (1. ..=10000.).contains(&layout.row_height)
            && layout.text_size.is_finite()
            && (1. ..=256.).contains(&layout.text_size),
        "invalid geographic symbol legend",
    )?;
    layout.format.validate()
}
fn normalize(value: f64, domain: [f64; 2]) -> f64 {
    // Subtracting two finite nonnegative numbers cannot overflow.
    ((value - domain[0]) / (domain[1] - domain[0])).clamp(0., 1.)
}
fn radius(value: f64, domain: [f64; 2], o: &GeoSymbolOptions) -> f64 {
    if value == 0. {
        return o.zero_radius;
    }
    let t = normalize(value, domain);
    let [lo, hi] = o.radius_range;
    match o.size {
        GeoSymbolSize::Area => (lo * lo * (1. - t) + hi * hi * t).sqrt(),
        GeoSymbolSize::Radius => lo * (1. - t) + hi * t,
    }
}
fn quantize(value: f64, domain: [f64; 2], colors: &[Color]) -> Color {
    colors
        [((normalize(value, domain) * colors.len() as f64).floor() as usize).min(colors.len() - 1)]
}
fn push_mark(scene: &mut Scene, mark: Mark, custom: bool) {
    if custom {
        scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
    }
    scene.marks.push(mark);
}
fn push_label(scene: &mut Scene, label: Label, owner: Option<&str>, custom: bool) {
    if let Some(owner) = owner {
        scene.label_series.insert(scene.labels.len(), owner.into());
    }
    if custom {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(label);
}
fn text(text: String, x: f64, y: f64, color: Color, size: f64, anchor: TextAnchor) -> Label {
    Label {
        text,
        x,
        y,
        color,
        size,
        anchor,
        rotation: 0.,
    }
}
fn legend_title(scene: &mut Scene, layout: &GeoLegendOptions, custom: bool) -> f64 {
    let mut y = layout.position[1];
    if !layout.title.is_empty() {
        push_label(
            scene,
            text(
                layout.title.clone(),
                layout.position[0],
                y + layout.text_size,
                layout.text_color,
                layout.text_size,
                TextAnchor::Start,
            ),
            None,
            custom,
        );
        y += layout.text_size + 8.;
    }
    y
}

impl GeoMapLayout {
    /// Atomically append a symbol layer and its actual legends. The map fit is
    /// reused; failed inputs, identity collisions or frame budgets change nothing.
    pub fn add_symbols(&mut self, symbols: &[GeoSymbol], options: &GeoSymbolOptions) -> Result<()> {
        self.projection
            .add_symbols(&mut self.scene, symbols, options)
    }
    pub fn add_glyphs(
        &mut self,
        symbols: &[GeoSymbol],
        options: &GeoGlyphOptions,
    ) -> Result<GeoGlyphLayer> {
        self.projection
            .add_glyphs(&mut self.scene, symbols, options)
    }
}
impl GeoProjectionContext {
    /// Append to any same-size Scene using this map's exact fitted projection.
    /// At most 10,000 input symbols, 256 series and 32 size legend values.
    /// Existing custom paint order is preserved; new items append in layer order.
    pub fn add_symbols(
        &self,
        scene: &mut Scene,
        symbols: &[GeoSymbol],
        o: &GeoSymbolOptions,
    ) -> Result<()> {
        self.add_symbol_layer(scene, symbols, o, None).map(|_| ())
    }
    /// Same fitted context and checked layer pipeline, with actual shared D3
    /// glyph geometry. New glyphs use full-paint projection masks (also for
    /// concave Star/Wye); the original geometry/anchor is never clipped away.
    pub fn add_glyphs(
        &self,
        scene: &mut Scene,
        symbols: &[GeoSymbol],
        o: &GeoGlyphOptions,
    ) -> Result<GeoGlyphLayer> {
        ensure(
            o.rotation.is_finite()
                && o.rotation.abs() <= 1e6
                && o.hidden.len() <= 256
                && o.hidden
                    .iter()
                    .all(|s| !s.trim().is_empty() && s.len() <= 1024),
            "invalid geographic glyph rotation/hidden series",
        )?;
        // Validate shape and mode even for an empty or fully hidden source.
        glyph_at(o, [0., 0.], 1.)?;
        ensure(
            !matches!(o.shape, crate::GlyphShape::Stroked(_)) || o.symbols.stroke.is_some(),
            "stroked geographic glyph requires a stroke",
        )?;
        self.add_symbol_layer(scene, symbols, &o.symbols, Some(o))
    }
    fn add_symbol_layer(
        &self,
        scene: &mut Scene,
        symbols: &[GeoSymbol],
        o: &GeoSymbolOptions,
        glyph: Option<&GeoGlyphOptions>,
    ) -> Result<GeoGlyphLayer> {
        scene.validate()?;
        ensure(
            [scene.width, scene.height] == self.dimensions,
            "geographic overlay Scene dimensions differ from fitted projection",
        )?;
        ensure(
            symbols.len() <= MAX_GEO_SYMBOLS,
            "geographic symbol budget exceeded",
        )?;
        // Conic finite footprints are not azimuthal disks. Preserve the old
        // disk intersection/masking profile below; conics always mask full ink.
        let conic = matches!(
            self.view.projection,
            super::GeoProjection::ConicEqualArea(_)
                | super::GeoProjection::ConicEquidistant(_)
                | super::GeoProjection::ConicConformal(_)
                | super::GeoProjection::Albers
        );
        let conic_mask = if conic && (o.clip_to_projection || o.clip_labels_to_projection) {
            self.projection_clip_path(0.25, 4096)?
                .ok_or_else(|| crate::ChartError("missing finite conic footprint".into()))
                .map(Some)?
        } else {
            None
        };
        // Reserve existing Scene clip work before cloning each real conic mask.
        // Do not multiply the input count: missing/hidden/culled rows can skip ink.
        let mut conic_usage = (0usize, 0usize, 0usize);
        if conic_mask.is_some() {
            for clip in scene
                .label_clips
                .values()
                .chain(scene.marks.iter().filter_map(|m| m.clip_path.as_deref()))
            {
                let stats = clip.picking_stats()?;
                conic_usage.0 += clip.command_count();
                conic_usage.1 += stats.flattened_segments;
                conic_usage.2 += stats.work;
            }
        }
        let mut reserve_conic = |mask: &crate::ClipPath| -> Result<()> {
            let stats = mask.picking_stats()?;
            let next = (
                conic_usage.0.saturating_add(mask.command_count()),
                conic_usage.1.saturating_add(stats.flattened_segments),
                conic_usage.2.saturating_add(stats.work),
            );
            ensure(
                next.0 <= 200_000 && next.1 <= 200_000 && next.2 <= 4_000_000,
                "conic overlay cumulative clip command/picking budget exceeded",
            )?;
            conic_usage = next;
            Ok(())
        };
        let disk = if (o.clip_to_projection || o.clip_labels_to_projection) && !conic {
            let disk = self.projection_disk().ok_or_else(|| {
                crate::ChartError(
                    "projection-clipped symbols require an azimuthal projection".into(),
                )
            })?;
            disk_clip::validate(disk)?;
            Some(disk)
        } else {
            None
        };
        ensure(
            o.radius_range.iter().copied().all(radius_valid)
                && o.radius_range[0] <= o.radius_range[1]
                && radius_valid(o.zero_radius)
                && o.label_size.is_finite()
                && (1. ..=256.).contains(&o.label_size),
            "invalid geographic symbol size",
        )?;
        if let Some(stroke) = o.stroke {
            ensure(
                stroke.width.is_finite() && (0.000001..=10000.).contains(&stroke.width),
                "invalid geographic symbol stroke",
            )?;
        }
        o.value_format.validate()?;
        if let GeoMissingSymbol::Draw { radius, label, .. } = &o.missing {
            ensure(
                radius_valid(*radius) && label.len() <= 1024,
                "invalid missing geographic symbol",
            )?;
        }
        let mut ids = BTreeSet::new();
        let mut series = BTreeSet::new();
        let mut max = 0_f64;
        for symbol in symbols {
            ensure(
                !symbol.id.trim().is_empty()
                    && symbol.id.len() <= 990
                    && !symbol.series.trim().is_empty()
                    && symbol.series.len() <= 1024
                    && symbol.label.len() <= 1024
                    && ids.insert(symbol.id.as_str())
                    && symbol.value.is_none_or(|v| v.is_finite() && v >= 0.),
                "invalid geographic symbol identity/value",
            )?;
            valid_position(symbol.position)?;
            series.insert(symbol.series.as_str());
            max = max.max(symbol.value.unwrap_or(0.));
        }
        ensure(
            series.len() <= 256,
            "geographic symbol series budget exceeded",
        )?;
        let domain = o.domain.unwrap_or([0., if max > 0. { max } else { 1. }]);
        domain_valid(domain)?;
        let mut series_colors = BTreeMap::new();
        let color_domain = match &o.color {
            GeoSymbolColor::Series(colors) => {
                ensure(
                    !colors.is_empty() && colors.len() <= 256,
                    "invalid geographic symbol palette",
                )?;
                for (index, id) in series.iter().enumerate() {
                    series_colors.insert(*id, colors[index % colors.len()]);
                }
                domain
            }
            GeoSymbolColor::Fixed(_) => domain,
            GeoSymbolColor::Value {
                domain: custom,
                colors,
            } => {
                ensure(
                    !colors.is_empty() && colors.len() <= 64,
                    "geographic color bin budget exceeded",
                )?;
                let domain = custom.unwrap_or(domain);
                domain_valid(domain)?;
                domain
            }
        };
        if let Some(legend) = &o.size_legend {
            legend_valid(&legend.layout)?;
            ensure(
                legend.values.len() <= MAX_GEO_SYMBOL_LEGEND_VALUES
                    && legend.values.iter().all(|v| {
                        v.is_finite()
                            && *v >= 0.
                            && (*v == 0. || (domain[0]..=domain[1]).contains(v))
                    }),
                "invalid geographic size legend values",
            )?;
        }
        if let Some(legend) = &o.color_legend {
            legend_valid(legend)?;
        }
        let color_rows = if o.color_legend.is_some() {
            (match &o.color {
                GeoSymbolColor::Series(_) => series.len(),
                GeoSymbolColor::Fixed(_) => 1,
                GeoSymbolColor::Value { colors, .. } => colors.len(),
            }) + usize::from(matches!(o.missing, GeoMissingSymbol::Draw { .. }))
        } else {
            0
        };
        let size_rows = o.size_legend.as_ref().map_or(0, |l| l.values.len());
        let legend_rows = color_rows + size_rows;
        let legend_titles =
            usize::from(
                o.size_legend
                    .as_ref()
                    .is_some_and(|l| !l.layout.title.is_empty()),
            ) + usize::from(o.color_legend.as_ref().is_some_and(|l| !l.title.is_empty()));
        // Conservative source-union admission before allocating any overlay tracks.
        ensure(
            scene.marks.len() + symbols.len() + legend_rows <= MAX_MARKS
                && scene.labels.len()
                    + if o.labels { symbols.len() } else { 0 }
                    + legend_rows
                    + legend_titles
                    <= MAX_MARKS,
            "geographic overlay union budget exceeded",
        )?;
        let color = |symbol: &GeoSymbol| match &o.color {
            GeoSymbolColor::Series(_) => series_colors[symbol.series.as_str()],
            GeoSymbolColor::Fixed(color) => *color,
            GeoSymbolColor::Value { colors, .. } => {
                quantize(symbol.value.unwrap_or(0.), color_domain, colors)
            }
        };
        let mut next = scene.clone();
        let mut path_commands = next
            .marks
            .iter()
            .map(|m| match &m.geometry {
                Geometry::Path(path) => path.len(),
                _ => 0,
            })
            .sum::<usize>();
        let custom = !next.draw_order.is_empty();
        // Lexical IDs make source permutation preserve paint order as well as identity.
        let mut ordered: Vec<_> = symbols.iter().collect();
        ordered.sort_by(|a, b| a.id.cmp(&b.id));
        let source_indices = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect::<BTreeMap<_, _>>();
        let mut sites = vec![];
        for symbol in ordered {
            if glyph.is_some_and(|g| g.hidden.contains(&symbol.series)) {
                continue;
            }
            let (r, fill, formatted) = match symbol.value {
                Some(value) => (
                    radius(value, domain, o),
                    color(symbol),
                    o.value_format.format(value)?,
                ),
                None => match &o.missing {
                    GeoMissingSymbol::Skip => continue,
                    GeoMissingSymbol::Draw {
                        radius,
                        color,
                        label,
                    } => (*radius, *color, label.clone()),
                },
            };
            if r == 0. {
                ensure(
                    glyph.is_none()
                        || symbol.value.is_none_or(|v| {
                            v == 0.
                                || o.radius_range[0] == 0.
                                    && (v <= domain[0] || o.radius_range[1] == 0.)
                        }),
                    "positive geographic glyph radius mapping underflow",
                )?;
                continue;
            }
            let Some([x, y]) = self.project(symbol.position)? else {
                continue;
            };
            let extent = extent(o, glyph, r)?;
            let stroke = o
                .stroke
                .map_or(0., |s| s.width / 2. * if glyph.is_some() { 4. } else { 1. });
            if o.clip
                && (x + extent[2] + stroke < self.plot.x
                    || x + extent[0] - stroke > self.plot.x + self.plot.width
                    || y + extent[3] + stroke < self.plot.y
                    || y + extent[1] - stroke > self.plot.y + self.plot.height)
            {
                continue;
            }
            let tooltip = format!("{}: {formatted}", symbol.label);
            ensure(
                tooltip.len() <= 1024,
                "geographic symbol tooltip budget exceeded",
            )?;
            let typed = if let Some(g) = glyph {
                Some(glyph_at(g, [x, y], r)?)
            } else {
                None
            };
            let geometry = if let Some(g) = &typed {
                g.geometry.clone()
            } else if let Some(disk) = disk.filter(|_| o.clip_to_projection && o.stroke.is_none()) {
                let Some(geometry) = disk_clip::intersect(o.shape, [x, y], r, disk)? else {
                    continue;
                };
                geometry
            } else {
                o.shape.geometry(x, y, r)
            };
            if let Geometry::Path(path) = &geometry {
                path_commands += path.len();
                ensure(
                    path_commands <= 200_000,
                    "geographic overlay path command budget exceeded",
                )?;
            }
            let mark_id = format!("geo-symbol:{}:{}", symbol.id.len(), symbol.id);
            let mut mark = if let Some(g) = &typed {
                glyph_ink(g, &mark_id, fill, o.stroke)?
            } else {
                Mark::new(&mark_id, geometry, fill)
            };
            mark.series = symbol.series.clone();
            mark.label = tooltip;
            mark.value = symbol.value.unwrap_or(0.);
            if typed.is_none() {
                mark.stroke = o.stroke;
            }
            mark.clip = o.clip.then_some(self.plot);
            if let Some(disk) =
                disk.filter(|_| o.clip_to_projection && (o.stroke.is_some() || glyph.is_some()))
            {
                mark.clip_path = Some(Box::new(crate::ClipPath::circle(
                    disk.center[0],
                    disk.center[1],
                    disk.radius,
                )));
            }
            if o.clip_to_projection
                && let Some(mask) = &conic_mask
            {
                reserve_conic(mask)?;
                mark.clip_path = Some(Box::new(mask.clone()));
            }
            if let Some(typed) = typed {
                sites.push(GeoGlyphSite {
                    id: symbol.id.clone(),
                    source_index: source_indices[symbol.id.as_str()],
                    mark_id,
                    series: symbol.series.clone(),
                    label: symbol.label.clone(),
                    value: symbol.value,
                    formatted_value: formatted,
                    color: fill,
                    glyph: typed,
                });
            }
            push_mark(&mut next, mark, custom);
            if o.labels {
                if o.clip_labels_to_projection
                    && let Some(mask) = &conic_mask
                {
                    reserve_conic(mask)?;
                    next.label_clips.insert(next.labels.len(), mask.clone());
                }
                if let Some(disk) = disk.filter(|_| o.clip_labels_to_projection) {
                    next.label_clips.insert(
                        next.labels.len(),
                        crate::ClipPath::circle(disk.center[0], disk.center[1], disk.radius),
                    );
                }
                push_label(
                    &mut next,
                    text(
                        symbol.label.clone(),
                        x,
                        y + extent[1] - 4.,
                        o.label_color,
                        o.label_size,
                        TextAnchor::Middle,
                    ),
                    Some(&symbol.series),
                    custom,
                );
            }
        }
        if let Some(legend) = &o.size_legend {
            let layout = &legend.layout;
            let mut y = legend_title(&mut next, layout, custom);
            let mut max_r = 0_f64;
            for value in &legend.values {
                max_r = max_r.max(
                    extent(o, glyph, radius(*value, domain, o))?
                        .into_iter()
                        .map(f64::abs)
                        .fold(0_f64, f64::max),
                );
            }
            for (index, value) in legend.values.iter().enumerate() {
                let r = radius(*value, domain, o);
                let extent = extent(o, glyph, r)?
                    .into_iter()
                    .map(f64::abs)
                    .fold(0_f64, f64::max);
                let height = layout
                    .row_height
                    .max(2. * extent + 4.)
                    .max(layout.text_size + 4.);
                if glyph.is_none() || r > 0. {
                    let id = format!("geo-size-legend:{index}");
                    let center = [layout.position[0] + max_r, y + height / 2.];
                    let mut mark = if let Some(g) = glyph {
                        glyph_ink(&glyph_at(g, center, r)?, id, o.label_color, o.stroke)?
                    } else {
                        let mut m =
                            Mark::new(id, o.shape.geometry(center[0], center[1], r), o.label_color);
                        m.stroke = o.stroke;
                        m
                    };
                    mark.series.clear();
                    mark.label.clear();
                    mark.value = *value;
                    push_mark(&mut next, mark, custom);
                }
                push_label(
                    &mut next,
                    text(
                        layout.format.format(*value)?,
                        layout.position[0] + 2. * max_r + 8.,
                        y + height / 2. + layout.text_size / 3.,
                        layout.text_color,
                        layout.text_size,
                        TextAnchor::Start,
                    ),
                    None,
                    custom,
                );
                y += height;
            }
        }
        if let Some(layout) = &o.color_legend {
            let mut rows: Vec<(String, Color, Option<String>)> = match &o.color {
                GeoSymbolColor::Series(_) => series_colors
                    .iter()
                    .map(|(id, color)| ((*id).into(), *color, Some((*id).into())))
                    .collect(),
                GeoSymbolColor::Fixed(color) => vec![("Symbols".into(), *color, None)],
                GeoSymbolColor::Value { colors, .. } => {
                    let mut rows = vec![];
                    for (i, color) in colors.iter().enumerate() {
                        let lo = color_domain[0] * (1. - i as f64 / colors.len() as f64)
                            + color_domain[1] * (i as f64 / colors.len() as f64);
                        let t = (i + 1) as f64 / colors.len() as f64;
                        let hi = color_domain[0] * (1. - t) + color_domain[1] * t;
                        rows.push((
                            format!(
                                "[{}, {}{}",
                                layout.format.format(lo)?,
                                layout.format.format(hi)?,
                                if i + 1 == colors.len() { "]" } else { ")" }
                            ),
                            *color,
                            None,
                        ));
                    }
                    rows
                }
            };
            if let GeoMissingSymbol::Draw { color, label, .. } = &o.missing {
                rows.push((label.clone(), *color, None));
            }
            let mut y = legend_title(&mut next, layout, custom);
            for (index, (label, color, owner)) in rows.into_iter().enumerate() {
                let glyph_extent = if glyph.is_some() {
                    extent(o, glyph, 6.)?
                        .into_iter()
                        .map(f64::abs)
                        .fold(0_f64, f64::max)
                        + o.stroke.map_or(0., |s| s.width * 2.)
                } else {
                    6.
                };
                let height = layout
                    .row_height
                    .max(layout.text_size + 4.)
                    .max(if glyph.is_some() {
                        glyph_extent * 2. + 4.
                    } else {
                        16.
                    });
                let center = [layout.position[0] + glyph_extent, y + height / 2.];
                let mut mark = if let Some(g) = glyph {
                    glyph_ink(
                        &glyph_at(g, center, 6.)?,
                        format!("geo-color-legend:{index}"),
                        color,
                        o.stroke,
                    )?
                } else {
                    Mark::new(
                        format!("geo-color-legend:{index}"),
                        Geometry::Rect {
                            x: layout.position[0],
                            y: y + (height - 12.) / 2.,
                            width: 12.,
                            height: 12.,
                        },
                        color,
                    )
                };
                mark.series = owner.clone().unwrap_or_default();
                mark.label.clear();
                let hidden = owner
                    .as_ref()
                    .is_some_and(|id| glyph.is_some_and(|g| g.hidden.contains(id)));
                if !hidden {
                    push_mark(&mut next, mark, custom);
                    push_label(
                        &mut next,
                        text(
                            label.clone(),
                            layout.position[0]
                                + if glyph.is_some() {
                                    2. * glyph_extent + 8.
                                } else {
                                    20.
                                },
                            y + height / 2. + layout.text_size / 3.,
                            layout.text_color,
                            layout.text_size,
                            TextAnchor::Start,
                        ),
                        owner.as_deref(),
                        custom,
                    );
                }
                if let Some(id) = owner {
                    if let Some(existing) = next.legend.iter().find(|l| l.id == id) {
                        ensure(
                            existing.color == color && existing.label == label,
                            "geographic symbol legend conflicts with existing series",
                        )?;
                    } else {
                        next.legend.push(LegendItem { id, label, color });
                    }
                }
                y += height;
            }
        }
        next.validate()?;
        let layer = GeoGlyphLayer {
            sites,
            domain,
            color_domain,
            hidden: glyph.map_or_else(BTreeSet::new, |g| g.hidden.clone()),
        };
        if glyph.is_some() {
            layer.interaction_binding(&next)?;
        }
        *scene = next;
        Ok(layer)
    }
}
