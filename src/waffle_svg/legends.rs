use super::*;

/// Fixed author-supplied legend item boxes, not estimated text measurements.
/// `layout` controls placement; duplicated flow/align/wrap settings must agree.
#[derive(Clone, Debug, PartialEq)]
pub struct WaffleSvgLegendSpec {
    pub key: String,
    pub options: LegendOptions,
    pub layout: LegendLayoutOptions,
    pub item_size: TextSize,
}
impl Default for WaffleSvgLegendSpec {
    fn default() -> Self {
        Self {
            key: "legend".into(),
            options: Default::default(),
            layout: Default::default(),
            item_size: TextSize {
                width: 120.,
                height: 24.,
            },
        }
    }
}
impl WaffleSvgLegendSpec {
    pub fn new(
        key: impl Into<String>,
        options: LegendOptions,
        layout: LegendLayoutOptions,
        item_size: TextSize,
    ) -> Result<Self> {
        let value = Self {
            key: key.into(),
            options,
            layout,
            item_size,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        plan::key(&self.key)?;
        self.options.validate()?;
        self.layout.validate()?;
        self.item_size.validate()?;
        let flow_gap = if self.layout.direction == LegendDirection::Horizontal {
            self.layout.column_gap
        } else {
            self.layout.row_gap
        };
        ensure(
            self.options.direction == self.layout.direction
                && self.options.wrap == self.layout.wrap
                && self.options.align == self.layout.align
                && self.options.gap == flow_gap,
            "Waffle legend presentation and layout flow settings disagree",
        )?;
        ensure(
            self.item_size.width > 0. && self.item_size.height > 0.,
            "Waffle legend item box must be nonempty",
        )?;
        let symbol = if self.options.symbol == LegendSymbol::None {
            0.
        } else {
            self.options.size
        };
        let width = self.options.item_width.unwrap_or(self.item_size.width);
        ensure(
            width >= symbol
                && self.item_size.height
                    >= symbol.max(self.options.text.size * self.options.text.line_height),
            "Waffle legend fixed item box is smaller than its symbol or text line box",
        )
    }
}
#[derive(Clone, Debug)]
pub struct WaffleSvgLegendItem {
    source_index: usize,
    rect: LayoutRect,
    background_id: String,
    symbol_index: Option<usize>,
    label_index: usize,
}
impl WaffleSvgLegendItem {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn rect(&self) -> LayoutRect {
        self.rect
    }
    pub fn background_id(&self) -> &str {
        &self.background_id
    }
    pub fn symbol_index(&self) -> Option<usize> {
        self.symbol_index
    }
    pub fn label_index(&self) -> usize {
        self.label_index
    }
}
#[derive(Clone, Debug)]
pub struct WaffleSvgLegend {
    spec: WaffleSvgLegendSpec,
    scene: Scene,
    items: Vec<WaffleSvgLegendItem>,
}
impl WaffleSvgLegend {
    pub fn spec(&self) -> &WaffleSvgLegendSpec {
        &self.spec
    }
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn items(&self) -> &[WaffleSvgLegendItem] {
        &self.items
    }
}
pub(super) fn build(
    frame: &WaffleSvgFrame,
    spec: WaffleSvgLegendSpec,
    legend_index: usize,
) -> Result<WaffleSvgLegend> {
    spec.validate()?;
    let size = TextSize {
        width: spec.options.item_width.unwrap_or(spec.item_size.width),
        height: spec.item_size.height,
    };
    let measured = frame
        .scene()
        .legend
        .iter()
        .map(|item| LegendMeasuredItem {
            id: item.id.clone(),
            size,
        })
        .collect::<Vec<_>>();
    let layout = layout_legend(&measured, &spec.layout)?;
    let mut scene = frame.config().scene()?;
    scene.background = Color(0, 0, 0, 0);
    let mut items = vec![];
    let font = frame
        .scene()
        .typography
        .legend_font(&spec.options.text.font, spec.options.text.weight);
    for (index, (source, placed)) in frame.scene().legend.iter().zip(layout.items).enumerate() {
        let source_index = frame
            .group_by_id(&source.id)
            .ok_or_else(|| ChartError("Waffle legend source is missing".into()))?;
        let rect = placed.rect;
        let background_id = format!("waffle-svg-legend:{legend_index}:background:{index}");
        scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
        scene.marks.push(Mark::new(
            &background_id,
            Geometry::Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            Color(0, 0, 0, 0),
        ));
        let symbol = spec.options.symbol;
        let amount = if symbol == LegendSymbol::None {
            0.
        } else {
            spec.options.size
        };
        let symbol_index = if amount > 0. {
            let y = rect.y + (rect.height - amount) / 2.;
            let geometry = match symbol {
                LegendSymbol::Circle => Geometry::Circle {
                    x: rect.x + amount / 2.,
                    y: y + amount / 2.,
                    radius: amount / 2.,
                },
                LegendSymbol::Line => Geometry::Rect {
                    x: rect.x,
                    y: rect.y + (rect.height - 2.) / 2.,
                    width: amount,
                    height: 2.,
                },
                _ => Geometry::Rect {
                    x: rect.x,
                    y,
                    width: amount,
                    height: amount,
                },
            };
            let slot = scene.marks.len();
            scene.draw_order.push(DrawItem::Mark(slot));
            scene.marks.push(Mark::new(
                format!("waffle-svg-legend:{legend_index}:symbol:{index}"),
                geometry,
                spec.options.symbol_color.unwrap_or(source.color),
            ));
            Some(slot)
        } else {
            None
        };
        let label_index = scene.labels.len();
        scene.typography.labels.insert(
            label_index,
            FontOptions {
                family: Some(font.family.clone()),
                weight: Some(font.weight),
            },
        );
        scene.labels.push(Label {
            x: rect.x
                + if amount > 0. {
                    amount + spec.options.item_gap
                } else {
                    0.
                },
            y: rect.y + rect.height / 2. + spec.options.text.size * 0.35,
            text: legend_label(source, &spec.options)?,
            color: spec.options.text.color.unwrap_or(frame.config().theme.text),
            size: spec.options.text.size,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        scene.draw_order.push(DrawItem::Label(label_index));
        items.push(WaffleSvgLegendItem {
            source_index,
            rect,
            background_id,
            symbol_index,
            label_index,
        });
    }
    scene.validate()?;
    Ok(WaffleSvgLegend { spec, scene, items })
}
