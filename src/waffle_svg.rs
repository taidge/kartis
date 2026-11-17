//! Sealed Waffle source captures and literal SVG paint occurrences. Transparent
//! group Areas are presentation descriptors, never fabricated source Marks or
//! interaction sites. Existing colored `WaffleMode::Areas` remains independent.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};

use crate::scene::ensure;
use crate::*;
mod legends;
mod plan;
pub use legends::*;
pub use plan::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaffleSvgEmptyStyle {
    pub color: Color,
    pub opacity: f64,
    pub border_color: Color,
}
#[derive(Clone, Debug)]
pub struct WaffleSvgArea {
    source_index: usize,
    polygon_index: usize,
    id: String,
    geometry: Geometry,
}
impl WaffleSvgArea {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn polygon_index(&self) -> usize {
        self.polygon_index
    }
    /// Presentation-only leaf identity, not a canonical business Mark ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
}
#[derive(Clone, Debug)]
struct Capture {
    config: ChartConfig,
    source: Vec<RadialDatum>,
    options: WaffleLayoutOptions,
    layout: WaffleLayout,
    cell_marks: Vec<Option<usize>>,
    mark_cells: Vec<Option<usize>>,
    groups: BTreeMap<String, usize>,
    areas: Vec<WaffleSvgArea>,
    area_slots: Vec<Vec<usize>>,
    empty: WaffleSvgEmptyStyle,
}
#[derive(Clone, Debug)]
pub struct WaffleSvgFrame(Arc<Capture>);
impl PartialEq for WaffleSvgFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WaffleSvgFrame {}
impl WaffleSvgFrame {
    pub fn new(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
    ) -> Result<Self> {
        Self::with_factories(config, source, options, &mut WaffleFactories::default())
    }
    pub fn with_factories(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        factories: &mut WaffleFactories<'_>,
    ) -> Result<Self> {
        Self::build(config, source, options, factories, None, true)
    }
    pub fn with_cell_factory(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        cell: &mut impl FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry>,
    ) -> Result<Self> {
        Self::build(
            config,
            source,
            options,
            &mut WaffleFactories::default(),
            Some(cell),
            true,
        )
    }
    pub fn with_factories_and_cell_factory(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        factories: &mut WaffleFactories<'_>,
        cell: &mut impl FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry>,
    ) -> Result<Self> {
        Self::build(config, source, options, factories, Some(cell), true)
    }
    pub(crate) fn legacy(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        factories: &mut WaffleFactories<'_>,
        cell: Option<&mut WaffleSvgCellFactory<'_>>,
    ) -> Result<Self> {
        Self::build(config, source, options, factories, cell, false)
    }
    fn build(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        factories: &mut WaffleFactories<'_>,
        cell: Option<&mut WaffleSvgCellFactory<'_>>,
        nivo: bool,
    ) -> Result<Self> {
        let mut empty = None;
        let layout = {
            let WaffleFactories {
                color,
                formatted_value,
                border_color,
            } = factories;
            let has_color = color.is_some();
            let has_formatter = formatted_value.is_some();
            let has_border = border_color.is_some();
            let mut resolve_color = |context: &WaffleGroupColorContext<'_>| {
                color.as_deref_mut().expect("configured Waffle SVG color")(context)
            };
            let mut format = |value| {
                formatted_value
                    .as_deref_mut()
                    .expect("configured Waffle SVG formatter")(value)
            };
            let mut border = |context: &WaffleBorderColorContext<'_>| {
                let value = border_color
                    .as_deref_mut()
                    .expect("configured Waffle SVG border")(context)?;
                if matches!(context.target, WaffleBorderTarget::Empty(_)) {
                    empty = Some(value);
                }
                Ok(value)
            };
            let mut roles = WaffleFactories {
                color: if has_color {
                    Some(&mut resolve_color)
                } else {
                    None
                },
                formatted_value: if has_formatter {
                    Some(&mut format)
                } else {
                    None
                },
                border_color: if has_border { Some(&mut border) } else { None },
            };
            if nivo {
                crate::waffle_layout::waffle_svg_layout_with_factories(
                    config, source, options, &mut roles,
                )?
            } else {
                waffle_layout_with_factories(config, source, options, &mut roles)?
            }
        };
        let layout = if let Some(cell) = cell {
            let mut call = |context: &WaffleCellContext<'_>| cell(context);
            crate::waffle_layout::customize_cells(options, layout, &mut call)?
        } else {
            layout
        };
        let border_color = if let Some(color) = empty {
            color
        } else {
            options
                .border_color
                .resolve_validated(InheritedColorContext {
                    inherited: options.empty_color,
                    surface: options.empty_color,
                    backdrop: config.theme.background,
                    theme: config.theme.text,
                })?
        };
        Self::capture(
            config,
            source,
            options,
            layout,
            WaffleSvgEmptyStyle {
                color: options.empty_color,
                opacity: options.empty_opacity,
                border_color,
            },
        )
    }
    fn capture(
        config: &ChartConfig,
        source: &[RadialDatum],
        options: &WaffleLayoutOptions,
        layout: WaffleLayout,
        empty: WaffleSvgEmptyStyle,
    ) -> Result<Self> {
        layout.scene.validate()?;
        layout.interaction.validate(&layout.scene)?;
        ensure(
            layout.groups.len() == source.len(),
            "Waffle SVG source cardinality changed",
        )?;
        let slots: BTreeMap<_, _> = layout
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();
        let mut cell_marks = vec![];
        let mut mark_cells = vec![None; layout.scene.marks.len()];
        let mut classified = BTreeSet::new();
        for cell in &layout.cells {
            let slot = cell
                .mark_id
                .as_deref()
                .map(|id| {
                    slots
                        .get(id)
                        .copied()
                        .ok_or_else(|| ChartError("Waffle SVG cell Mark is missing".into()))
                })
                .transpose()?;
            if let Some(slot) = slot {
                ensure(classified.insert(slot), "duplicate Waffle SVG cell Mark")?;
                mark_cells[slot] = Some(cell.index);
            }
            cell_marks.push(slot);
        }
        let mut groups = BTreeMap::new();
        let mut areas = vec![];
        let mut area_slots = vec![];
        for (index, group) in layout.groups.iter().enumerate() {
            ensure(
                group.source_index == index
                    && groups.insert(group.source.id.clone(), index).is_none(),
                "invalid Waffle SVG source identity",
            )?;
            if let Some(id) = &group.area_mark_id {
                let slot = *slots
                    .get(id.as_str())
                    .ok_or_else(|| ChartError("Waffle SVG union Mark missing".into()))?;
                ensure(classified.insert(slot), "duplicate Waffle SVG union Mark")?;
            }
            let mut indices = vec![];
            if !group.hidden {
                for (polygon_index, polygon) in group.polygons.iter().enumerate() {
                    let mut commands = polygon
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            if i == 0 {
                                C::Move(P::new(p[0], p[1]))
                            } else {
                                C::Line(P::new(p[0], p[1]))
                            }
                        })
                        .collect::<Vec<_>>();
                    if !polygon.is_empty() {
                        commands.push(C::Close);
                    }
                    indices.push(areas.len());
                    areas.push(WaffleSvgArea {
                        source_index: index,
                        polygon_index,
                        id: format!("waffle-svg-area:{index}:{polygon_index}"),
                        geometry: Geometry::Path(commands),
                    });
                }
            }
            area_slots.push(indices);
        }
        ensure(
            classified.len() == layout.scene.marks.len() && layout.scene.labels.is_empty(),
            "unclassified Waffle SVG source paint",
        )?;
        Ok(Self(Arc::new(Capture {
            config: config.clone(),
            source: source.to_vec(),
            options: options.clone(),
            layout,
            cell_marks,
            mark_cells,
            groups,
            areas,
            area_slots,
            empty,
        })))
    }
    pub fn config(&self) -> &ChartConfig {
        &self.0.config
    }
    pub fn source(&self) -> &[RadialDatum] {
        &self.0.source
    }
    pub fn options(&self) -> &WaffleLayoutOptions {
        &self.0.options
    }
    pub fn layout(&self) -> &WaffleLayout {
        &self.0.layout
    }
    pub fn scene(&self) -> &Scene {
        &self.0.layout.scene
    }
    pub fn plot(&self) -> ClipRect {
        self.0.layout.plot()
    }
    pub fn groups(&self) -> &[WaffleGroup] {
        &self.0.layout.groups
    }
    pub fn cells(&self) -> &[WaffleCell] {
        &self.0.layout.cells
    }
    pub fn areas(&self) -> &[WaffleSvgArea] {
        &self.0.areas
    }
    pub fn empty_style(&self) -> WaffleSvgEmptyStyle {
        self.0.empty
    }
    pub fn group_by_id(&self, id: &str) -> Option<usize> {
        self.0.groups.get(id).copied()
    }
    pub fn cell_mark(&self, index: usize) -> Result<Option<&Mark>> {
        Ok(self
            .0
            .cell_marks
            .get(index)
            .ok_or_else(|| ChartError("Waffle SVG cell index missing".into()))?
            .map(|slot| &self.scene().marks[slot]))
    }
    pub fn cell_for_mark(&self, index: usize) -> Result<Option<usize>> {
        self.0
            .mark_cells
            .get(index)
            .copied()
            .ok_or_else(|| ChartError("Waffle SVG Mark index missing".into()))
    }
    pub fn area(&self, source_index: usize, polygon_index: usize) -> Result<&WaffleSvgArea> {
        let slots = self
            .0
            .area_slots
            .get(source_index)
            .ok_or_else(|| ChartError("Waffle SVG area source index missing".into()))?;
        let index = *slots
            .get(polygon_index)
            .ok_or_else(|| ChartError("Waffle SVG polygon index missing".into()))?;
        Ok(&self.0.areas[index])
    }
    pub fn interaction(&self) -> SceneInteraction {
        self.layout().clone().into_scene_interaction()
    }
}
type WaffleSvgCellFactory<'a> =
    dyn FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry> + 'a;
