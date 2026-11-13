use super::WidgetRenderer;
use crate::{
    ChartError, ClipPath, ClipRect, Geometry, LayoutRect, Mark, Result, Scene, TextSize,
    ZoomTransform, clip_path::RectCoverage, scene::ensure,
};
use glory::render::Semantics;
use glory::style::{Display, Length, Overflow, PointerEvents, Position, Profile, Style};
use glory::{Bond, Cage, Scope, Vary, Widget};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Maximum simultaneous measured slots for a semantic chart.
pub const MAX_PLOT_WIDGET_SLOTS: usize = 128;

/// A caller-measured overlay placement in Scene coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlotWidgetPlacement {
    Rect(LayoutRect),
    /// Offset of the slot's top-left corner from its datum anchor.
    /// Use an explicit rectangle for Paths without a typed BrowserChart anchor.
    AtDatum {
        offset: [f64; 2],
        measured: TextSize,
    },
}

/// Attach one measured Widget slot to a real chart datum.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetSlot {
    pub mark_id: String,
    /// A new revision asks for a new child while retaining the keyed wrapper.
    pub revision: u64,
    pub placement: PlotWidgetPlacement,
}

/// An application-owned set of measured slots in paint order. The optional
/// rectangle constrains slots to the plot; omission uses the whole Scene.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlotWidgetsLayout {
    pub plot: Option<ClipRect>,
    pub slots: Vec<PlotWidgetSlot>,
}

/// Current accepted context for one slot. Partial slots paint through their
/// clip and are inert, so clipped controls cannot take focus.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotWidgetFrame {
    pub mark: Mark,
    /// Rendered Scene coordinates. BrowserChart applies its current camera
    /// before this rectangle is normalized to the responsive surface box.
    pub rect: LayoutRect,
    /// Visible rectangle, or a conservative rectangular envelope of a
    /// partial BrowserChart path clip. The host draws the actual mask.
    pub visible: Option<LayoutRect>,
    pub interactive: bool,
}

/// The read-only frame updates without replacing the mounted Widget.
#[derive(Clone)]
pub struct PlotWidgetContext {
    pub mark_id: String,
    pub revision: u64,
    pub frame: Bond<PlotWidgetFrame>,
}

pub(crate) type PlotWidgetFactory = Box<dyn FnMut(PlotWidgetContext) -> Result<Box<dyn Widget>>>;

pub(super) struct PlotWidgetsInput {
    pub layout: Vary<PlotWidgetsLayout>,
    pub factory: PlotWidgetFactory,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedSlot {
    revision: u64,
    mark: Mark,
    rect: LayoutRect,
    clip: Option<LayoutRect>,
    // In BrowserChart, percentages resolve against the actual (possibly
    // nonuniformly resized) CSS paint box independently of canvas DPR.
    relative_to: Option<[f64; 2]>,
    mask: Option<PreparedSlotMask>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreparedSlotMask {
    index: usize,
    /// Scale and translation into the clip wrapper's object bounding box.
    transform: [f64; 4],
}

impl PreparedSlot {
    fn frame(&self, hidden: &BTreeSet<String>) -> PlotWidgetFrame {
        let visible = self
            .clip
            .filter(|_| self.mark.opacity > 0. && !hidden.contains(&self.mark.series));
        PlotWidgetFrame {
            mark: self.mark.clone(),
            rect: self.rect,
            visible,
            interactive: self.mask.is_none() && visible == Some(self.rect),
        }
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn path_mask(&self) -> Option<(usize, [f64; 4], &ClipPath)> {
        let mask = self.mask?;
        Some((mask.index, mask.transform, self.mark.clip_path.as_deref()?))
    }

    fn clip_class(&self, owner: Option<&str>) -> String {
        match (self.mask, owner) {
            (Some(mask), Some(owner)) => {
                format!("kartis-plot-widget-clip {owner}-slot-{}", mask.index)
            }
            _ => "kartis-plot-widget-clip".into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PlotWidgetsState {
    pub rows: Cage<Vec<PreparedSlot>>,
    pub profile: Profile,
    factory: WidgetRenderer<PlotWidgetContext>,
    mask_owner: Option<Rc<str>>,
}
impl PlotWidgetsState {
    pub fn new(
        scope: &Scope,
        rows: Vec<PreparedSlot>,
        profile: Profile,
        factory: PlotWidgetFactory,
    ) -> Self {
        Self {
            rows: scope.cage(rows),
            profile,
            factory: WidgetRenderer::new(Some(factory)),
            mask_owner: None,
        }
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub fn with_mask_owner(mut self, owner: Rc<str>) -> Self {
        self.mask_owner = Some(owner);
        self
    }

    pub fn clear(&self) {
        self.factory.clear();
    }

    pub fn show(
        self,
        scope: &Scope,
        style: impl Into<Vary<Style>>,
        hidden: Cage<BTreeSet<String>>,
        reject: impl Fn(ChartError) + Clone + 'static,
    ) -> glory::ui::Result {
        let rows = self.rows;
        let factory = self.factory;
        let mask_owner = self.mask_owner;
        glory::ui::view()
            .class("kartis-plot-widgets")
            .style(style)
            .fill(
                glory::ui::each(rows)
                    .key(|row| row.mark.id.clone())
                    .children(move |row| {
                        let factory = factory.clone();
                        let reject = reject.clone();
                        let mask_owner = mask_owner.clone();
                        glory::ui::widget(move |scope| {
                            let id = row.key().clone();
                            let datum = row.value;
                            let frame = scope.bond(move || {
                                datum.with(|datum| hidden.with(|hidden| datum.frame(hidden)))
                            });
                            let child_factory = factory.clone();
                            let child_rejection = reject.clone();
                            let mask_owner = mask_owner.clone();
                            glory::ui::view()
                                .class(scope.bond(move || {
                                    datum.with(|slot| slot.clip_class(mask_owner.as_deref()))
                                }))
                                .style(scope.bond(move || {
                                    frame.with(|frame| {
                                        datum.with(|slot| clip_style(frame, slot.relative_to))
                                    })
                                }))
                                .semantics(scope.bond(move || {
                                    frame.with(|frame| {
                                        Some(Box::new(Semantics {
                                            hidden: frame.visible.is_none(),
                                            inert: !frame.interactive,
                                            ..Default::default()
                                        }))
                                    })
                                }))
                                .fill(
                                    glory::ui::view()
                                        .class("kartis-plot-widget-slot")
                                        .style(scope.bond(move || {
                                            frame.with(|frame| {
                                                datum.with(|slot| {
                                                    content_style(frame, slot.relative_to.is_some())
                                                })
                                            })
                                        }))
                                        .fill(glory::ui::widget(move |scope| {
                                            let weak = scope.downgrade();
                                            let mut attempted_revision = None;
                                            let mut mounted = None;
                                            let child_factory = child_factory.clone();
                                            let child_rejection = child_rejection.clone();
                                            let id = id.clone();
                                            scope.structural(move || {
                                                let revision = datum.with(|datum| datum.revision);
                                                if attempted_revision == Some(revision) {
                                                    return;
                                                }
                                                attempted_revision = Some(revision);
                                                if mounted.as_ref().is_some_and(|(accepted, _)| {
                                                    *accepted == revision
                                                }) {
                                                    return;
                                                }
                                                let Some(scope) = weak.upgrade() else { return };
                                                let result = glory::core::untracked(|| {
                                                    let child =
                                                        child_factory.call(PlotWidgetContext {
                                                            mark_id: id.clone(),
                                                            revision,
                                                            frame,
                                                        })?;
                                                    scope.mount_with(|| child).map_err(|error| {
                                                        ChartError(error.to_string())
                                                    })
                                                });
                                                match result {
                                                    Ok(next) => {
                                                        drop(mounted.replace((revision, next)))
                                                    }
                                                    Err(error) => child_rejection(error),
                                                }
                                            });
                                            Ok(())
                                        })),
                                )
                                .show_in(scope)
                        })
                    }),
            )
            .show_in(scope)
    }
}

pub(super) fn prepare(
    scene: &Scene,
    layout: &PlotWidgetsLayout,
    profile: Profile,
    view: ZoomTransform,
) -> Result<Vec<PreparedSlot>> {
    view.validate()?;
    prepare_in_space(
        scene,
        layout,
        profile,
        SlotSpace {
            plot: None,
            transform: view,
            responsive: false,
            require_current_data: true,
        },
        |mark| Ok(crate::crosshair::mark_center(mark)),
        |_| None,
    )
}

pub(crate) struct SlotSpace {
    pub plot: Option<ClipRect>,
    pub transform: ZoomTransform,
    pub responsive: bool,
    pub require_current_data: bool,
}

pub(crate) fn prepare_in_space(
    scene: &Scene,
    layout: &PlotWidgetsLayout,
    profile: Profile,
    space: SlotSpace,
    anchor: impl Fn(&Mark) -> Result<Option<[f64; 2]>>,
    value: impl Fn(&Mark) -> Option<f64>,
) -> Result<Vec<PreparedSlot>> {
    ensure(
        layout.slots.len() <= MAX_PLOT_WIDGET_SLOTS,
        "plot Widget slot budget exceeded",
    )?;
    crate::surface::validate_plot(layout.plot, scene)?;
    let relative_to = space.responsive.then_some([scene.width, scene.height]);
    profile_style(profile, &overlay_style(scene, space.responsive))?;
    let screen = LayoutRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
    };
    let plot = space.plot.map(layout_rect).unwrap_or(screen);
    let view = layout
        .plot
        .map_or(Some(plot), |clip| intersect(plot, layout_rect(clip)));
    let marks: BTreeMap<_, _> = scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), mark))
        .collect();
    let mut ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(layout.slots.len());
    for (index, slot) in layout.slots.iter().enumerate() {
        ensure(
            !slot.mark_id.is_empty() && slot.mark_id.len() <= 1024,
            "invalid plot Widget datum ID",
        )?;
        ensure(
            ids.insert(slot.mark_id.as_str()),
            "duplicate plot Widget datum",
        )?;
        let Some(mark) = marks
            .get(slot.mark_id.as_str())
            .filter(|mark| !mark.label.is_empty())
        else {
            ensure(
                !space.require_current_data,
                "plot Widget slot requires a current data mark",
            )?;
            continue;
        };
        let rect = match slot.placement {
            PlotWidgetPlacement::Rect(rect) => rect,
            PlotWidgetPlacement::AtDatum { offset, measured } => {
                let [x, y] = anchor(mark)?.ok_or_else(|| ChartError(
                    "a Path datum without an interaction anchor requires an explicit plot Widget rectangle".into(),
                ))?;
                LayoutRect {
                    x: x + offset[0],
                    y: y + offset[1],
                    width: measured.width,
                    height: measured.height,
                }
            }
        };
        rect.validate()?;
        ensure(
            rect.width > 0.
                && rect.height > 0.
                && rect.x + rect.width > rect.x
                && rect.y + rect.height > rect.y,
            "plot Widget slot requires positive representable measured dimensions",
        )?;
        let rect = project_rect(rect, space.transform)?;
        let clip = view.and_then(|view| intersect(rect, view));
        let clip = if let Some(mark_clip) = mark.clip {
            let mark_clip = project_rect(layout_rect(mark_clip), space.transform)?;
            clip.and_then(|visible| intersect(visible, mark_clip))
        } else {
            clip
        };
        let (clip, mask) = if let (Some(clip), Some(mask)) = (clip, mark.clip_path.as_deref()) {
            path_mask(clip, mask, space.transform, index)?
        } else {
            (clip, None)
        };
        ensure(
            space.responsive || mask.is_none(),
            "partial plot Widget path masks require the browser mask host",
        )?;
        let mut mark = (*mark).clone();
        if let Some(value) = value(&mark) {
            mark.value = value;
        }
        let row = PreparedSlot {
            revision: slot.revision,
            mark,
            rect,
            clip,
            relative_to,
            mask,
        };
        let frame = row.frame(&BTreeSet::new());
        profile_style(profile, &clip_style(&frame, relative_to))?;
        profile_style(profile, &content_style(&frame, space.responsive))?;
        rows.push(row);
    }
    // Hidden/offscreen transitions use the same finite style properties.
    let hidden = LayoutRect::default();
    profile_style(profile, &positioned(hidden, None).display(Display::None))?;
    Ok(rows)
}

fn path_mask(
    clip: LayoutRect,
    mask: &ClipPath,
    view: ZoomTransform,
    index: usize,
) -> Result<(Option<LayoutRect>, Option<PreparedSlotMask>)> {
    if let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = mask.geometry
    {
        let bounds = project_rect(
            LayoutRect {
                x,
                y,
                width,
                height,
            },
            view,
        )?;
        return Ok((intersect(clip, bounds), None));
    }
    let (x, y) = view.unproject((clip.x, clip.y))?;
    let world = LayoutRect {
        x,
        y,
        width: clip.width / view.scale,
        height: clip.height / view.scale,
    };
    match mask.rect_coverage(world)? {
        RectCoverage::Outside => Ok((None, None)),
        RectCoverage::Full => Ok((Some(clip), None)),
        RectCoverage::Partial => {
            let transform = [
                view.scale / clip.width,
                view.scale / clip.height,
                (view.translate_x - clip.x) / clip.width,
                (view.translate_y - clip.y) / clip.height,
            ];
            ensure(
                transform
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 1e9),
                "plot Widget path mask normalization exceeds coordinate budget",
            )?;
            Ok((Some(clip), Some(PreparedSlotMask { index, transform })))
        }
    }
}

fn project_rect(rect: LayoutRect, transform: ZoomTransform) -> Result<LayoutRect> {
    let (x, y) = transform.project((rect.x, rect.y))?;
    let width = rect.width * transform.scale;
    let height = rect.height * transform.scale;
    ensure(
        width.is_finite()
            && height.is_finite()
            && (x + width).is_finite()
            && (y + height).is_finite(),
        "invalid projected plot Widget rectangle",
    )?;
    Ok(LayoutRect {
        x,
        y,
        width,
        height,
    })
}

fn profile_style(profile: Profile, style: &Style) -> Result<()> {
    style.validate().map_err(|error| ChartError(error.into()))?;
    profile.inline.check(style).map_err(|error| {
        ChartError(format!(
            "plot Widget layout is unsupported on {} ({error:?})",
            profile.name
        ))
    })
}

fn layout_rect(rect: ClipRect) -> LayoutRect {
    LayoutRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn intersect(a: LayoutRect, b: LayoutRect) -> Option<LayoutRect> {
    if b.contains(a) {
        return Some(a);
    }
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then_some(LayoutRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

pub(crate) fn overlay_style(scene: &Scene, responsive: bool) -> Style {
    let [width, height] = if responsive {
        [Length::Percent(100.), Length::Percent(100.)]
    } else {
        [Length::Px(scene.width), Length::Px(scene.height)]
    };
    Style::new()
        .position(Position::Absolute)
        .left(Length::Px(0.))
        .top(Length::Px(0.))
        .width(width)
        .height(height)
        .pointer_events(PointerEvents::None)
}

fn positioned(rect: LayoutRect, relative_to: Option<[f64; 2]>) -> Style {
    let [left, top, width, height] = if let Some([width, height]) = relative_to {
        [
            Length::Percent(rect.x / width * 100.),
            Length::Percent(rect.y / height * 100.),
            Length::Percent(rect.width / width * 100.),
            Length::Percent(rect.height / height * 100.),
        ]
    } else {
        [
            Length::Px(rect.x),
            Length::Px(rect.y),
            Length::Px(rect.width),
            Length::Px(rect.height),
        ]
    };
    Style::new()
        .position(Position::Absolute)
        .left(left)
        .top(top)
        .width(width)
        .height(height)
}

fn clip_style(frame: &PlotWidgetFrame, relative_to: Option<[f64; 2]>) -> Style {
    positioned(frame.visible.unwrap_or_default(), relative_to)
        .display(if frame.visible.is_some() {
            Display::Block
        } else {
            Display::None
        })
        .overflow_x(Overflow::Hidden)
        .overflow_y(Overflow::Hidden)
        .pointer_events(PointerEvents::None)
}

fn content_style(frame: &PlotWidgetFrame, responsive: bool) -> Style {
    let visible = frame.visible.unwrap_or(frame.rect);
    positioned(
        LayoutRect {
            x: frame.rect.x - visible.x,
            y: frame.rect.y - visible.y,
            ..frame.rect
        },
        responsive.then_some([visible.width, visible.height]),
    )
    .overflow_x(Overflow::Hidden)
    .overflow_y(Overflow::Hidden)
    .pointer_events(if frame.interactive {
        PointerEvents::Auto
    } else {
        PointerEvents::None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChartConfig, Color, Geometry};

    #[test]
    fn path_mask_classifies_holes_and_conservatively_admits_interaction() {
        let rectangle = |x, y, width, height| LayoutRect {
            x,
            y,
            width,
            height,
        };
        let path = |rect: LayoutRect| {
            Geometry::Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            }
            .commands()
        };
        let mut commands = path(rectangle(10., 10., 90., 90.));
        commands.extend(path(rectangle(40., 40., 20., 20.)));
        let mask = ClipPath {
            geometry: Geometry::Path(commands),
            rule: crate::ClipRule::EvenOdd,
        };
        let view = ZoomTransform::new(2., -10., 5.).unwrap();
        let project = |r| project_rect(r, view).unwrap();
        let outside = project(rectangle(46., 44., 8., 10.));
        assert_eq!(path_mask(outside, &mask, view, 0).unwrap(), (None, None));
        let whole = project(rectangle(25., 25., 12., 12.));
        assert_eq!(
            path_mask(whole, &mask, view, 0).unwrap(),
            (Some(whole), None)
        );
        let partial = project(rectangle(33., 45., 20., 10.));
        let (clip, normalized) = path_mask(partial, &mask, view, 2).unwrap();
        assert_eq!(clip, Some(partial));
        assert_eq!(normalized.unwrap().transform, [0.05, 0.1, -1.65, -4.5]);
        assert_eq!(normalized.unwrap().index, 2);

        let circle = ClipPath::circle(20., 20., 10.);
        let disjoint = project(rectangle(28., 28., 2., 2.));
        assert_eq!(path_mask(disjoint, &circle, view, 0).unwrap(), (None, None));
        let crossing = project(rectangle(24., 24., 5., 5.));
        assert!(path_mask(crossing, &circle, view, 0).unwrap().1.is_some());
    }

    #[test]
    fn responsive_slots_project_the_mark_clip_and_use_a_fixed_plot_aperture() {
        let mut scene = ChartConfig {
            width: 320.,
            height: 160.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        let mut mark = Mark::new(
            "alpha",
            Geometry::Circle {
                x: 80.,
                y: 60.,
                radius: 10.,
            },
            Color(30, 70, 90, 255),
        );
        mark.clip = Some(ClipRect {
            x: 72.,
            y: 53.,
            width: 20.,
            height: 14.,
        });
        scene.marks.push(mark);
        let layout = PlotWidgetsLayout {
            plot: None,
            slots: vec![PlotWidgetSlot {
                mark_id: "alpha".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-10., -5.],
                    measured: TextSize {
                        width: 20.,
                        height: 10.,
                    },
                },
            }],
        };
        let rows = prepare_in_space(
            &scene,
            &layout,
            Profile::HTML,
            SlotSpace {
                plot: Some(ClipRect {
                    x: 120.,
                    y: 70.,
                    width: 90.,
                    height: 75.,
                }),
                transform: ZoomTransform::new(2., -30., -20.).unwrap(),
                responsive: true,
                require_current_data: true,
            },
            |mark| Ok(crate::crosshair::mark_center(mark)),
            |_| Some(7.),
        )
        .unwrap();
        let row = &rows[0];
        let frame = row.frame(&BTreeSet::new());
        assert_eq!(frame.mark.value, 7.);
        assert_eq!(
            frame.rect,
            LayoutRect {
                x: 110.,
                y: 90.,
                width: 40.,
                height: 20.
            }
        );
        assert_eq!(
            frame.visible,
            Some(LayoutRect {
                x: 120.,
                y: 90.,
                width: 30.,
                height: 20.
            })
        );
        assert!(!frame.interactive);
        let clip = clip_style(&frame, row.relative_to);
        assert_eq!(clip.left, Some(Length::Percent(37.5)));
        assert_eq!(clip.width, Some(Length::Percent(9.375)));
        assert_eq!(clip.overflow_x, Some(Overflow::Hidden));
        let content = content_style(&frame, true);
        assert_eq!(content.left, Some(Length::Percent(-10. / 30. * 100.)));
        assert_eq!(content.width, Some(Length::Percent(40. / 30. * 100.)));
        assert_eq!(content.pointer_events, Some(PointerEvents::None));
    }
}
