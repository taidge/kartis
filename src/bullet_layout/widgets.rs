//! Typed, measured Glory Widget overlays for actual Bullet parts.
use super::*;

/// Owned metadata from the accepted Bullet layout. The mounted Widget receives
/// this alongside its current paint and placement in `TypedPlotWidgetFrame`.
#[derive(Clone, Debug)]
pub struct BulletPartWidgetData {
    pub part: BulletPart,
    pub row_index: usize,
    /// Rank within this kind, including zero or cropped parts without marks.
    pub rank: usize,
    pub domain: [f64; 2],
    pub track: ClipRect,
}

// Source signed zero is observable metadata even when geometry is unchanged.
impl PartialEq for BulletPartWidgetData {
    fn eq(&self, other: &Self) -> bool {
        self.part == other.part
            && crate::widget_source_eq::number(self.part.raw_value, other.part.raw_value)
            // The start can be an original endpoint from an omitted zero part.
            // Visible/clipped coordinates retain ordinary geometry equality.
            && self.part.interval.iter().zip(&other.part.interval).all(|(a, b)| {
                crate::widget_source_eq::number(*a, *b)
            })
            && self.row_index == other.row_index
            && self.rank == other.rank
            && self.domain == other.domain
            && self.track == other.track
    }
}

/// The original layout and mark supplied to the synchronous placement callback.
#[derive(Clone, Copy, Debug)]
pub struct BulletPartWidgetContext<'a> {
    pub row: &'a BulletRowLayout,
    pub row_index: usize,
    pub part: &'a BulletPart,
    pub rank: usize,
    pub mark: &'a Mark,
}

/// Build measured slots and owned metadata together, without changing the Scene.
/// Only actual range, measure and marker marks invoke `placement`; omitted zero
/// or cropped parts still count toward their kind's rank. Returning `None`
/// leaves a part without an overlay. Slots retain the source mark identity.
///
/// Feed this packet to `Chart::plot_widgets_with_data` or the corresponding
/// BrowserChart methods. A stable revision retains the mounted child while its
/// accepted frame and metadata update. Dispatch on `frame.data.part.kind` to
/// use separate range, measure and marker Widget components. This is an overlay:
/// the original Bullet ink remains, and Scene exports omit live child Widgets.
///
/// Placement callbacks are borrowed and are not retained. Failure returns no
/// packet; external callback side effects remain application-owned. The host
/// validates placements, clip support and the 128-slot budget before admission.
///
/// ```rust
/// use glory::{App, Widget};
/// use kartis::*;
/// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// let source = [BulletDatum {
///     id: "orders".into(),
///     ranges: vec![30., 90.],
///     measures: vec![65.],
///     markers: vec![80.],
/// }];
/// let layout = bullet_layout(&ChartConfig::default(), &source, &Default::default())?;
/// let slots = bullet_part_widgets(&layout, 0, |part| {
///     Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
///         x: part.row.track.x,
///         y: part.row.track.y,
///         width: 100.,
///         height: 24.,
///     })))
/// })?;
/// let app = App::memory();
/// app.mount(
///     chart(layout.scene).plot_widgets_with_data(slots, |context| {
///         let frame = context.frame;
///         Ok(glory::ui::widget(move |scope| {
///             glory::ui::button()
///                 .label(scope.bond(move || {
///                     frame.with(|current| {
///                         format!(
///                             "{:?}: {}",
///                             current.data.part.kind, current.data.part.raw_value
///                         )
///                     })
///                 }))
///                 .show_in(scope)
///         }))
///     }),
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn bullet_part_widgets(
    layout: &BulletLayout,
    revision: u64,
    mut placement: impl FnMut(&BulletPartWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<BulletPartWidgetData>> {
    layout.scene.validate()?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut rows = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout::default(),
        data: BTreeMap::new(),
    };
    for (row_index, row) in layout.rows.iter().enumerate() {
        ensure(rows.insert(&row.id), "duplicate Bullet Widget row")?;
        ensure(
            row.domain.iter().all(|v| v.is_finite()) && row.domain[0] <= row.domain[1],
            "invalid Bullet Widget row domain",
        )?;
        crate::surface::validate_plot(Some(row.track), &layout.scene)?;
        let mut ranks = [0; 3];
        for part in &row.parts {
            ensure(parts.insert(&part.id), "duplicate Bullet Widget part")?;
            ensure(
                part.row == row.id
                    && part.raw_value.is_finite()
                    && part
                        .interval
                        .iter()
                        .chain(&part.visible_interval)
                        .all(|v| v.is_finite()),
                "invalid Bullet Widget part metadata",
            )?;
            let kind = match part.kind {
                BulletPartKind::Range => 0,
                BulletPartKind::Measure => 1,
                BulletPartKind::Marker => 2,
            };
            let rank = ranks[kind];
            ranks[kind] += 1;
            let Some(mark) = marks.get(part.id.as_str()).copied() else {
                continue;
            };
            ensure(
                mark.series == row.id
                    && mark.value == part.raw_value
                    && mark.geometry == part.geometry,
                "Bullet Widget part differs from its current mark",
            )?;
            let context = BulletPartWidgetContext {
                row,
                row_index,
                part,
                rank,
                mark,
            };
            if let Some(placement) = placement(&context)? {
                ensure(
                    packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                    "plot Widget slot budget exceeded",
                )?;
                packet.layout.slots.push(PlotWidgetSlot {
                    mark_id: part.id.clone(),
                    revision,
                    placement,
                });
                packet.data.insert(
                    part.id.clone(),
                    BulletPartWidgetData {
                        part: part.clone(),
                        row_index,
                        rank,
                        domain: row.domain,
                        track: row.track,
                    },
                );
            }
        }
    }
    Ok(packet)
}
