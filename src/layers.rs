//! Named, checked Rust paint layers and explicitly computed custom content.
//!
//! A layer plan references one computed Scene by frame-local `DrawItem` indices.
//! Every existing item must occur exactly once. Custom content can interleave
//! geometry and text, and is validated together with the final scene before
//! publication. Callbacks run only when the application explicitly calls
//! `CustomLayer::compute`; no callback is retained or scheduled by this module.
use crate::scene::{ensure, *};
use std::collections::BTreeSet;

pub const MAX_CHART_LAYERS: usize = 128;
fn name_valid(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 128
}

#[derive(Clone, Debug, PartialEq)]
pub struct NamedLayer {
    pub name: String,
    pub items: Vec<DrawItem>,
}
impl NamedLayer {
    /// References are checked against the source frame when the plan composes.
    pub fn new(name: impl Into<String>, items: Vec<DrawItem>) -> Result<Self> {
        let name = name.into();
        ensure(
            name_valid(&name) && items.len() <= MAX_MARKS * 2,
            "invalid named layer or item budget",
        )?;
        Ok(Self { name, items })
    }
    /// Preserve the source scene's effective paint order in a named base layer.
    pub fn all(name: impl Into<String>, scene: &Scene) -> Result<Self> {
        scene.validate()?;
        Self::new(name, scene.draw_items())
    }
}
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)] // Public value-based custom layer API.
pub enum LayerContent {
    Mark(Mark),
    Label {
        label: Label,
        series: Option<String>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct CustomLayer {
    pub name: String,
    /// Content order is paint order, including geometry/text interleaving.
    pub content: Vec<LayerContent>,
}
impl CustomLayer {
    /// Validate an independent proposal. Composition additionally checks IDs
    /// against all existing marks and every other custom layer.
    pub fn new(name: impl Into<String>, content: Vec<LayerContent>) -> Result<Self> {
        let layer = Self {
            name: name.into(),
            content,
        };
        ensure(
            name_valid(&layer.name) && layer.content.len() <= MAX_MARKS * 2,
            "invalid custom layer or item budget",
        )?;
        let mut frame = ChartConfig::default().scene()?;
        append_custom(&mut frame, &layer)?;
        frame.validate()?;
        Ok(layer)
    }
    /// Compute a custom layer once from a validated, immutable source frame.
    pub fn compute(
        scene: &Scene,
        name: impl Into<String>,
        compute: impl FnOnce(&Scene) -> Result<Vec<LayerContent>>,
    ) -> Result<Self> {
        scene.validate()?;
        let name = name.into();
        ensure(name_valid(&name), "invalid custom layer name")?;
        Self::new(name, compute(scene)?)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ChartLayer {
    References(NamedLayer),
    Custom(CustomLayer),
}
impl ChartLayer {
    pub fn name(&self) -> &str {
        match self {
            Self::References(layer) => &layer.name,
            Self::Custom(layer) => &layer.name,
        }
    }
}
impl From<NamedLayer> for ChartLayer {
    fn from(layer: NamedLayer) -> Self {
        Self::References(layer)
    }
}
impl From<CustomLayer> for ChartLayer {
    fn from(layer: CustomLayer) -> Self {
        Self::Custom(layer)
    }
}

fn valid_names(layers: &[ChartLayer]) -> Result<()> {
    ensure(
        layers.len() <= MAX_CHART_LAYERS,
        "chart layer budget exceeded",
    )?;
    let mut names = BTreeSet::new();
    ensure(
        layers
            .iter()
            .all(|layer| name_valid(layer.name()) && names.insert(layer.name())),
        "invalid or duplicate chart layer name",
    )
}
fn append_custom(frame: &mut Scene, layer: &CustomLayer) -> Result<()> {
    let marks = layer
        .content
        .iter()
        .filter(|item| matches!(item, LayerContent::Mark(_)))
        .count();
    let labels = layer.content.len() - marks;
    ensure(
        frame
            .marks
            .len()
            .checked_add(marks)
            .is_some_and(|n| n <= MAX_MARKS)
            && frame
                .labels
                .len()
                .checked_add(labels)
                .is_some_and(|n| n <= MAX_MARKS),
        "custom layer scene budget exceeded",
    )?;
    for item in &layer.content {
        match item {
            LayerContent::Mark(mark) => {
                frame.draw_order.push(DrawItem::Mark(frame.marks.len()));
                frame.marks.push(mark.clone());
            }
            LayerContent::Label { label, series } => {
                let index = frame.labels.len();
                frame.draw_order.push(DrawItem::Label(index));
                frame.labels.push(label.clone());
                if let Some(series) = series {
                    frame.label_series.insert(index, series.clone());
                }
            }
        }
    }
    Ok(())
}
/// Compose a complete paint sequence. Named references address only original
/// source items; they cannot accidentally point into newly appended content.
pub fn compose_layers(scene: &Scene, layers: &[ChartLayer]) -> Result<Scene> {
    scene.validate()?;
    valid_names(layers)?;
    let (mut marks, mut labels) = (scene.marks.len(), scene.labels.len());
    let mut references = 0usize;
    for layer in layers {
        match layer {
            ChartLayer::References(layer) => {
                references = references
                    .checked_add(layer.items.len())
                    .ok_or_else(|| ChartError("layer reference budget overflow".into()))?;
                ensure(
                    references <= scene.marks.len() + scene.labels.len(),
                    "too many source layer references",
                )?;
                ensure(
                    layer.items.iter().all(|item| match *item {
                        DrawItem::Mark(i) => i < scene.marks.len(),
                        DrawItem::Label(i) => i < scene.labels.len(),
                    }),
                    "layer references an absent source item",
                )?;
            }
            ChartLayer::Custom(layer) => {
                ensure(
                    layer.content.len() <= MAX_MARKS * 2,
                    "custom layer item budget exceeded",
                )?;
                for item in &layer.content {
                    match item {
                        LayerContent::Mark(_) => {
                            marks = marks
                                .checked_add(1)
                                .ok_or_else(|| ChartError("layer mark count overflow".into()))?
                        }
                        LayerContent::Label { .. } => {
                            labels = labels
                                .checked_add(1)
                                .ok_or_else(|| ChartError("layer label count overflow".into()))?
                        }
                    }
                }
            }
        }
    }
    ensure(
        references == scene.marks.len() + scene.labels.len(),
        "layer plan must reference every source item exactly once",
    )?;
    ensure(
        marks <= MAX_MARKS && labels <= MAX_MARKS,
        "composed layer scene budget exceeded",
    )?;
    let mut next = scene.clone();
    next.draw_order.clear();
    for layer in layers {
        match layer {
            ChartLayer::References(layer) => next.draw_order.extend_from_slice(&layer.items),
            ChartLayer::Custom(layer) => append_custom(&mut next, layer)?,
        }
    }
    next.validate()?;
    Ok(next)
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerPlan {
    pub layers: Vec<ChartLayer>,
}
impl LayerPlan {
    pub fn for_scene(scene: &Scene) -> Result<Self> {
        Ok(Self {
            layers: vec![NamedLayer::all("chart", scene)?.into()],
        })
    }
    pub fn from_layers(layers: Vec<ChartLayer>) -> Result<Self> {
        valid_names(&layers)?;
        Ok(Self { layers })
    }
    pub fn push(&mut self, layer: impl Into<ChartLayer>) -> Result<()> {
        self.insert(self.layers.len(), layer.into())
    }
    fn insert(&mut self, index: usize, layer: ChartLayer) -> Result<()> {
        valid_names(&self.layers)?;
        ensure(
            self.layers.len() < MAX_CHART_LAYERS
                && name_valid(layer.name())
                && !self.layers.iter().any(|old| old.name() == layer.name()),
            "duplicate layer name or layer budget exceeded",
        )?;
        self.layers.insert(index, layer);
        Ok(())
    }
    pub fn insert_before(&mut self, anchor: &str, layer: impl Into<ChartLayer>) -> Result<()> {
        let index = self
            .layers
            .iter()
            .position(|layer| layer.name() == anchor)
            .ok_or_else(|| ChartError("unknown chart layer anchor".into()))?;
        self.insert(index, layer.into())
    }
    pub fn insert_after(&mut self, anchor: &str, layer: impl Into<ChartLayer>) -> Result<()> {
        let index = self
            .layers
            .iter()
            .position(|layer| layer.name() == anchor)
            .ok_or_else(|| ChartError("unknown chart layer anchor".into()))?;
        self.insert(index + 1, layer.into())
    }
    pub fn compose(&self, scene: &Scene) -> Result<Scene> {
        compose_layers(scene, &self.layers)
    }
    /// Atomically replace the frame only after all layers have been admitted.
    pub fn apply(&self, scene: &mut Scene) -> Result<()> {
        let next = self.compose(scene)?;
        *scene = next;
        Ok(())
    }
}
