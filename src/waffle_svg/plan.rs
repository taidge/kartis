use super::*;

pub const MAX_WAFFLE_SVG_LAYERS: usize = 128;
pub const MAX_WAFFLE_SVG_WORK: usize = 200_000;
pub const MAX_WAFFLE_SVG_SOURCE_COMMANDS: usize = 2_000_000;
pub const MAX_WAFFLE_SVG_COMPONENTS: usize = 128;
pub const MAX_WAFFLE_SVG_KEY_BYTES: usize = 128;
pub(super) fn key(value: &str) -> Result<()> {
    ensure(
        !value.trim().is_empty() && value.len() <= MAX_WAFFLE_SVG_KEY_BYTES,
        "invalid Waffle SVG key",
    )
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaffleSvgLayerKind {
    Cells,
    Areas,
    Legends,
    Custom(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaffleSvgLayer {
    pub key: String,
    pub kind: WaffleSvgLayerKind,
}
impl WaffleSvgLayer {
    pub fn new(id: impl Into<String>, kind: WaffleSvgLayerKind) -> Result<Self> {
        let layer = Self {
            key: id.into(),
            kind,
        };
        layer.validate()?;
        Ok(layer)
    }
    fn validate(&self) -> Result<()> {
        key(&self.key)?;
        if let WaffleSvgLayerKind::Custom(registry) = &self.kind {
            key(registry)?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaffleSvgBudget {
    pub layers: usize,
    pub work: usize,
    pub source_commands: usize,
    pub components: usize,
}
impl Default for WaffleSvgBudget {
    fn default() -> Self {
        Self {
            layers: MAX_WAFFLE_SVG_LAYERS,
            work: MAX_WAFFLE_SVG_WORK,
            source_commands: MAX_WAFFLE_SVG_SOURCE_COMMANDS,
            components: MAX_WAFFLE_SVG_COMPONENTS,
        }
    }
}
impl WaffleSvgBudget {
    pub fn validate(self) -> Result<()> {
        ensure(
            self.layers <= MAX_WAFFLE_SVG_LAYERS
                && self.work <= MAX_WAFFLE_SVG_WORK
                && self.source_commands <= MAX_WAFFLE_SVG_SOURCE_COMMANDS
                && self.components <= MAX_WAFFLE_SVG_COMPONENTS,
            "Waffle SVG budget exceeds hard limits",
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaffleSvgItem {
    Cell {
        cell_index: usize,
        mark_index: usize,
    },
    Area {
        source_index: usize,
        polygon_index: usize,
    },
    Legend {
        legend_index: usize,
        item_index: usize,
    },
}
#[derive(Clone, Debug)]
pub struct WaffleSvgOccurrence {
    pub index: usize,
    pub layer: WaffleSvgLayer,
    items: Vec<WaffleSvgItem>,
}
impl WaffleSvgOccurrence {
    pub fn items(&self) -> &[WaffleSvgItem] {
        &self.items
    }
}
#[derive(Clone, Debug)]
struct Plan {
    frame: WaffleSvgFrame,
    layers: Vec<WaffleSvgOccurrence>,
    legends: Vec<WaffleSvgLegend>,
    budget: WaffleSvgBudget,
    work: usize,
    source_commands: usize,
}
#[derive(Clone, Debug)]
pub struct WaffleSvgPlan(Arc<Plan>);
impl PartialEq for WaffleSvgPlan {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WaffleSvgPlan {}
impl WaffleSvgPlan {
    pub fn default_layers() -> Vec<WaffleSvgLayer> {
        vec![
            WaffleSvgLayer {
                key: "cells".into(),
                kind: WaffleSvgLayerKind::Cells,
            },
            WaffleSvgLayer {
                key: "areas".into(),
                kind: WaffleSvgLayerKind::Areas,
            },
            WaffleSvgLayer {
                key: "legends".into(),
                kind: WaffleSvgLayerKind::Legends,
            },
        ]
    }
    pub fn defaults(frame: &WaffleSvgFrame) -> Result<Self> {
        Self::new(frame, Self::default_layers(), vec![], Default::default())
    }
    pub fn validate_layers(layers: &[WaffleSvgLayer], budget: WaffleSvgBudget) -> Result<()> {
        budget.validate()?;
        ensure(
            layers.len() <= budget.layers,
            "Waffle SVG occurrence budget exceeded",
        )?;
        let mut keys = BTreeSet::new();
        let mut components = 0usize;
        for layer in layers {
            layer.validate()?;
            ensure(
                keys.insert(layer.key.as_str()),
                "duplicate Waffle SVG occurrence key",
            )?;
            components += usize::from(matches!(layer.kind, WaffleSvgLayerKind::Custom(_)));
        }
        ensure(
            components <= budget.components,
            "Waffle SVG component occurrence budget exceeded",
        )
    }
    pub fn new(
        frame: &WaffleSvgFrame,
        layers: Vec<WaffleSvgLayer>,
        legends: Vec<WaffleSvgLegendSpec>,
        budget: WaffleSvgBudget,
    ) -> Result<Self> {
        Self::validate_layers(&layers, budget)?;
        ensure(
            frame.options().mode == WaffleMode::Cells,
            "literal Waffle SVG layers require Cells mode; colored area unions remain a separate legacy API",
        )?;
        ensure(
            legends.len() <= MAX_WAFFLE_SVG_LAYERS,
            "Waffle SVG legend count exceeded",
        )?;
        let has_legends = layers.iter().any(|l| l.kind == WaffleSvgLayerKind::Legends);
        let mut legend_keys = BTreeSet::new();
        let mut prepared = vec![];
        for (i, spec) in legends.into_iter().enumerate() {
            spec.validate()?;
            ensure(
                legend_keys.insert(spec.key.clone()),
                "duplicate Waffle SVG legend key",
            )?;
            if has_legends {
                prepared.push(legends::build(frame, spec, i)?);
            }
        }
        let mut occurrences = vec![];
        let mut work = 0usize;
        let mut source_commands = 0usize;
        for (index, layer) in layers.into_iter().enumerate() {
            let items = match &layer.kind {
                WaffleSvgLayerKind::Cells => frame
                    .scene()
                    .draw_items()
                    .into_iter()
                    .map(|item| match item {
                        DrawItem::Mark(mark_index) => Ok(WaffleSvgItem::Cell {
                            cell_index: frame.cell_for_mark(mark_index)?.ok_or_else(|| {
                                ChartError("unclassified Waffle SVG cell paint".into())
                            })?,
                            mark_index,
                        }),
                        DrawItem::Label(_) => {
                            Err(ChartError("unclassified Waffle SVG label paint".into()))
                        }
                    })
                    .collect::<Result<Vec<_>>>()?,
                WaffleSvgLayerKind::Areas => frame
                    .areas()
                    .iter()
                    .map(|a| WaffleSvgItem::Area {
                        source_index: a.source_index(),
                        polygon_index: a.polygon_index(),
                    })
                    .collect(),
                WaffleSvgLayerKind::Legends => prepared
                    .iter()
                    .enumerate()
                    .flat_map(|(legend_index, l)| {
                        l.items().iter().enumerate().map(move |(item_index, _)| {
                            WaffleSvgItem::Legend {
                                legend_index,
                                item_index,
                            }
                        })
                    })
                    .collect(),
                WaffleSvgLayerKind::Custom(_) => vec![],
            };
            add(
                &mut work,
                items.len() + usize::from(matches!(layer.kind, WaffleSvgLayerKind::Custom(_))),
                budget.work,
                "Waffle SVG work budget exceeded",
            )?;
            for item in &items {
                let cost = match *item {
                    WaffleSvgItem::Cell { mark_index, .. } => {
                        mark_cost(&frame.scene().marks[mark_index])?
                    }
                    WaffleSvgItem::Area {
                        source_index,
                        polygon_index,
                    } => geometry_cost(frame.area(source_index, polygon_index)?.geometry()),
                    WaffleSvgItem::Legend {
                        legend_index,
                        item_index,
                    } => {
                        let legend = &prepared[legend_index];
                        let row = &legend.items()[item_index];
                        let mut cost = 5usize;
                        if let Some(i) = row.symbol_index() {
                            cost = cost
                                .checked_add(mark_cost(&legend.scene().marks[i])?)
                                .ok_or_else(|| {
                                    ChartError("Waffle SVG legend resource overflow".into())
                                })?;
                        }
                        cost.checked_add(
                            1 + legend.scene().labels[row.label_index()].text.len()
                                + legend
                                    .scene()
                                    .resolved_label_font(row.label_index())
                                    .signature()
                                    .len(),
                        )
                        .ok_or_else(|| ChartError("Waffle SVG legend text overflow".into()))?
                    }
                };
                add(
                    &mut source_commands,
                    cost,
                    budget.source_commands,
                    "Waffle SVG source resource budget exceeded",
                )?;
            }
            occurrences.push(WaffleSvgOccurrence {
                index,
                layer,
                items,
            });
        }
        let plan = Self(Arc::new(Plan {
            frame: frame.clone(),
            layers: occurrences,
            legends: prepared,
            budget,
            work,
            source_commands,
        }));
        // Every sealed builtin projection must itself be a legal Scene, before
        // a browser can construct any custom runtime for this plan.
        for occurrence in plan.layers() {
            if !matches!(occurrence.layer.kind, WaffleSvgLayerKind::Custom(_)) {
                plan.native_scene(occurrence.index)?;
            }
        }
        Ok(plan)
    }
    pub fn frame(&self) -> &WaffleSvgFrame {
        &self.0.frame
    }
    pub fn layers(&self) -> &[WaffleSvgOccurrence] {
        &self.0.layers
    }
    pub fn legends(&self) -> &[WaffleSvgLegend] {
        &self.0.legends
    }
    pub fn budget(&self) -> WaffleSvgBudget {
        self.0.budget
    }
    pub fn work(&self) -> usize {
        self.0.work
    }
    pub fn source_commands(&self) -> usize {
        self.0.source_commands
    }
    pub fn native_scene(&self, index: usize) -> Result<Scene> {
        let occurrence = self
            .layers()
            .get(index)
            .ok_or_else(|| ChartError("Waffle SVG occurrence index missing".into()))?;
        let mut scene = self.frame().config().scene()?;
        scene.background = Color(0, 0, 0, 0);
        for item in occurrence.items() {
            match *item {
                WaffleSvgItem::Cell { mark_index, .. } => scene
                    .marks
                    .push(self.frame().scene().marks[mark_index].clone()),
                WaffleSvgItem::Area {
                    source_index,
                    polygon_index,
                } => {
                    let area = self.frame().area(source_index, polygon_index)?;
                    scene.marks.push(Mark::new(
                        area.id(),
                        area.geometry().clone(),
                        Color(0, 0, 0, 0),
                    ));
                }
                WaffleSvgItem::Legend {
                    legend_index,
                    item_index,
                } => {
                    let legend = &self.legends()[legend_index];
                    let row = &legend.items()[item_index];
                    let background = legend
                        .scene()
                        .marks
                        .iter()
                        .find(|m| m.id == row.background_id())
                        .ok_or_else(|| ChartError("Waffle SVG legend background missing".into()))?;
                    scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
                    scene.marks.push(background.clone());
                    if let Some(i) = row.symbol_index() {
                        scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
                        scene.marks.push(legend.scene().marks[i].clone());
                    }
                    let slot = scene.labels.len();
                    let font = legend.scene().resolved_label_font(row.label_index());
                    scene.typography.labels.insert(
                        slot,
                        FontOptions {
                            family: Some(font.family),
                            weight: Some(font.weight),
                        },
                    );
                    scene.draw_order.push(DrawItem::Label(slot));
                    scene
                        .labels
                        .push(legend.scene().labels[row.label_index()].clone());
                }
            }
        }
        if occurrence.layer.kind == WaffleSvgLayerKind::Cells {
            scene.opacity_groups = self.frame().scene().opacity_groups.clone();
        }
        scene.validate()?;
        Ok(scene)
    }
    pub fn native_hit_ids(&self, index: usize) -> Result<BTreeSet<String>> {
        let occurrence = self
            .layers()
            .get(index)
            .ok_or_else(|| ChartError("Waffle SVG occurrence index missing".into()))?;
        occurrence
            .items()
            .iter()
            .map(|item| {
                Ok(match *item {
                    WaffleSvgItem::Cell { mark_index, .. } => {
                        self.frame().scene().marks[mark_index].id.clone()
                    }
                    WaffleSvgItem::Area {
                        source_index,
                        polygon_index,
                    } => self
                        .frame()
                        .area(source_index, polygon_index)?
                        .id()
                        .to_owned(),
                    WaffleSvgItem::Legend {
                        legend_index,
                        item_index,
                    } => self.legends()[legend_index].items()[item_index]
                        .background_id()
                        .to_owned(),
                })
            })
            .collect()
    }
}
fn add(total: &mut usize, amount: usize, limit: usize, error: &str) -> Result<()> {
    *total = total
        .checked_add(amount)
        .ok_or_else(|| ChartError(error.into()))?;
    ensure(*total <= limit, error)
}
fn geometry_cost(g: &Geometry) -> usize {
    g.commands().len()
}
fn mark_cost(mark: &Mark) -> Result<usize> {
    let mut cost = geometry_cost(&mark.geometry);
    for amount in [
        usize::from(mark.clip.is_some()) * 5,
        mark.clip_path
            .as_ref()
            .map_or(0, |c| geometry_cost(&c.geometry)),
        mark.stroke_dash.as_ref().map_or(0, |d| d.segments.len()),
        mark.fill_paint.as_ref().map_or(0, |p| {
            if matches!(p, FillPaint::Pattern(_)) {
                32
            } else {
                p.stops().len() + 1
            }
        }),
    ] {
        cost = cost
            .checked_add(amount)
            .ok_or_else(|| ChartError("Waffle SVG resource work overflow".into()))?;
    }
    Ok(cost)
}
