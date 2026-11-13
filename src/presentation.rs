//! Renderer-independent legend layout and safe, bounded tooltip text.
//! Strings are plain text. Consumers must never interpret them as HTML.
use crate::scene::ensure;
use crate::{
    Color, LegendItem, LegendRole, LegendTarget, MAX_LABEL_BYTES, Mark, Result, ValueFormat,
};
use glory::style::{
    Align, Display, Edges, FlexDirection, FlexWrap, Justify, Length, Style, WhiteSpace,
};
use std::collections::BTreeMap;

/// Reactive inputs for an application-supplied legend content/symbol Widget.
/// Returned Widgets provide content, not nested interactive controls. Chart
/// supplies a series toggle, a configured category action, or static content.
/// Category visibility never follows the hidden-series set.
pub struct LegendRenderContext {
    pub item: glory::Bond<LegendItem>,
    pub hidden: glory::Bond<bool>,
    /// Current accepted legend domain; Category is not a series toggle.
    pub role: glory::Bond<LegendRole>,
    /// Current resolved business target, distinct from the keyed display row.
    pub target: glory::Bond<LegendTarget>,
    pub label: glory::Bond<String>,
}
/// A legend activation resolves its original ID, label, color and domain from
/// the currently accepted Scene. The item ID is a transport row identity;
/// its optional explicit target carries the original category or canonical
/// series ID. Category actions never hide series.
/// This is an activation intent snapshot, not confirmation of hidden-state
/// publication. Chart notifies before toggling; queued hosts recheck identity
/// role and target when processing the intent.
#[derive(Clone, PartialEq)]
pub struct LegendActivation {
    pub item: LegendItem,
    pub role: LegendRole,
    /// Explicit target snapshot from the accepted frame. None preserves the
    /// legacy role plus item-ID contract and its original Debug output.
    pub target: Option<LegendTarget>,
}
impl LegendActivation {
    /// Resolve this immutable intent snapshot without consulting a later Scene.
    /// Hosts must recheck it against the accepted row before acting on it.
    pub fn resolved_target(&self) -> LegendTarget {
        self.target.clone().unwrap_or_else(|| match self.role {
            LegendRole::Series => LegendTarget::Series(self.item.id.clone()),
            LegendRole::Category => LegendTarget::Category(self.item.id.clone()),
        })
    }
}
impl std::fmt::Debug for LegendActivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = f.debug_struct("LegendActivation");
        output.field("item", &self.item).field("role", &self.role);
        if self.target.is_some() {
            output.field("target", &self.target);
        }
        output.finish()
    }
}
/// Visible and accessible text shared by legend controls and measurement.
/// Series items retain the original Show/Hide operation; categories are labels.
pub fn legend_control_label(
    item: &LegendItem,
    role: LegendRole,
    hidden: bool,
    options: &LegendOptions,
) -> Result<String> {
    let label = legend_label(item, options)?;
    Ok(if role == LegendRole::Series {
        format!("{} {label}", if hidden { "Show" } else { "Hide" })
    } else {
        label
    })
}
/// Bindings resolve the currently accepted, visible data, including after a
/// scene update. A factory runs once per mounted tooltip, not per pointer event.
pub struct TooltipRenderContext {
    pub marks: glory::Bond<Vec<Mark>>,
    pub text: glory::Bond<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendDirection {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationAlign {
    #[default]
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendSymbol {
    None,
    #[default]
    Square,
    Circle,
    Line,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PresentationText {
    pub font: crate::FontOptions,
    pub color: Option<Color>,
    pub size: f64,
    pub weight: f64,
    /// Line height as a ratio of font size.
    pub line_height: f64,
}
impl Default for PresentationText {
    fn default() -> Self {
        Self {
            font: crate::FontOptions::default(),
            color: None,
            size: 13.,
            weight: 400.,
            line_height: 1.4,
        }
    }
}
impl PresentationText {
    /// Resolve standalone presentation text while retaining its legacy weight.
    /// Chart role inheritance instead uses SceneTypography::legend_font or
    /// tooltip_font to distinguish an explicit Scene weight from that default.
    pub fn resolved_font(&self, parent: &crate::ResolvedFont) -> crate::ResolvedFont {
        self.font.resolve(&crate::ResolvedFont {
            weight: self.weight,
            ..parent.clone()
        })
    }

    /// Supply the same resolved selection to a text measurer and the host style.
    pub fn with_resolved_font(&self, font: &crate::ResolvedFont) -> Self {
        Self {
            font: crate::FontOptions {
                family: Some(font.family.clone()),
                weight: Some(font.weight),
            },
            weight: font.weight,
            ..self.clone()
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.font.validate()?;
        ensure(
            self.size.is_finite() && (1. ..=512.).contains(&self.size),
            "invalid presentation font size",
        )?;
        ensure(
            self.weight.is_finite() && (1. ..=1000.).contains(&self.weight),
            "invalid presentation font weight",
        )?;
        ensure(
            self.line_height.is_finite() && (0.5..=10.).contains(&self.line_height),
            "invalid presentation line height",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct LegendOptions {
    pub direction: LegendDirection,
    pub wrap: bool,
    pub gap: f64,
    pub item_gap: f64,
    /// Horizontal rows justify along their main axis; vertical stacks align
    /// across their horizontal axis. Wrapping follows available host space.
    pub align: PresentationAlign,
    pub item_width: Option<f64>,
    pub symbol: LegendSymbol,
    pub size: f64,
    pub symbol_color: Option<Color>,
    pub text: PresentationText,
    pub series_labels: BTreeMap<String, String>,
    pub label_prefix: String,
    pub label_suffix: String,
}
impl Default for LegendOptions {
    fn default() -> Self {
        Self {
            direction: LegendDirection::Horizontal,
            wrap: true,
            gap: 16.,
            item_gap: 6.,
            align: PresentationAlign::Start,
            item_width: None,
            symbol: LegendSymbol::Square,
            size: 12.,
            symbol_color: None,
            text: PresentationText::default(),
            series_labels: BTreeMap::new(),
            label_prefix: String::new(),
            label_suffix: String::new(),
        }
    }
}
fn text_budget(text: &str) -> Result<()> {
    ensure(
        text.len() <= MAX_LABEL_BYTES,
        "presentation text budget exceeded",
    )
}
fn labels_budget(labels: &BTreeMap<String, String>) -> Result<()> {
    ensure(
        labels.len() <= 256,
        "presentation series label count exceeded",
    )?;
    for (id, label) in labels {
        ensure(!id.is_empty(), "empty presentation series identity")?;
        text_budget(id)?;
        text_budget(label)?;
    }
    Ok(())
}
impl LegendOptions {
    pub fn validate(&self) -> Result<()> {
        self.text.validate()?;
        labels_budget(&self.series_labels)?;
        text_budget(&self.label_prefix)?;
        text_budget(&self.label_suffix)?;
        ensure(
            [self.gap, self.item_gap, self.size]
                .iter()
                .all(|v| v.is_finite() && (0. ..=4096.).contains(v)),
            "invalid legend spacing/symbol size",
        )?;
        ensure(
            self.item_width
                .is_none_or(|v| v.is_finite() && (1. ..=16_384.).contains(&v)),
            "invalid legend item width",
        )
    }
}
pub fn legend_label(item: &LegendItem, options: &LegendOptions) -> Result<String> {
    options.validate()?;
    let label = options.series_labels.get(&item.id).unwrap_or(&item.label);
    let label = format!("{}{label}{}", options.label_prefix, options.label_suffix);
    text_budget(&label)?;
    Ok(label)
}
pub fn text_style(options: &PresentationText) -> Style {
    Style {
        font_family: options.font.family.clone(),
        color: options.color,
        font_size: Some(options.size),
        font_weight: Some(options.font.weight.unwrap_or(options.weight)),
        line_height: Some(options.line_height * options.size),
        ..Style::new()
    }
}
pub fn legend_style(options: &LegendOptions) -> Style {
    let justify = match options.align {
        PresentationAlign::Start => Justify::Start,
        PresentationAlign::Center => Justify::Center,
        PresentationAlign::End => Justify::End,
    };
    Style {
        align: Some(if options.direction == LegendDirection::Horizontal {
            Align::Center
        } else {
            match options.align {
                PresentationAlign::Start => Align::Start,
                PresentationAlign::Center => Align::Center,
                PresentationAlign::End => Align::End,
            }
        }),
        justify: Some(if options.direction == LegendDirection::Horizontal {
            justify
        } else {
            Justify::Start
        }),
        ..Style::new()
            .display(Display::Flex)
            .flex_direction(match options.direction {
                LegendDirection::Horizontal => FlexDirection::Row,
                LegendDirection::Vertical => FlexDirection::Column,
            })
            .flex_wrap(if options.wrap {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            })
            .gap(options.gap)
    }
}
pub fn legend_item_style(options: &LegendOptions) -> Style {
    Style {
        width: options.item_width.map(Length::Px),
        align: Some(Align::Center),
        justify: Some(Justify::Start),
        ..text_style(&options.text)
            .display(Display::InlineFlex)
            .gap(options.item_gap)
            .white_space(WhiteSpace::NoWrap)
    }
}
pub fn legend_symbol_style(options: &LegendOptions, color: Color, hidden: bool) -> Style {
    let height = if options.symbol == LegendSymbol::Line {
        2.
    } else {
        options.size
    };
    Style {
        background: Some(options.symbol_color.unwrap_or(color)),
        radius: Some(if options.symbol == LegendSymbol::Circle {
            options.size / 2.
        } else {
            0.
        }),
        shrink: Some(0.),
        ..Style::new()
            .display(if options.symbol == LegendSymbol::None {
                Display::None
            } else {
                Display::InlineFlex
            })
            .width(Length::Px(options.size))
            .height(Length::Px(height))
            .opacity(if hidden { 0.35 } else { 1. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TooltipPart {
    Text(String),
    Label,
    Series,
    Value,
    Id,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MissingValue {
    #[default]
    HideTooltip,
    OmitRow,
    Placeholder(String),
}
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipOptions {
    pub lines: Vec<Vec<TooltipPart>>,
    pub heading: Option<String>,
    pub value_format: ValueFormat,
    pub series_labels: BTreeMap<String, String>,
    pub missing: MissingValue,
    pub max_rows: usize,
    pub overflow_text: Option<String>,
    pub text: PresentationText,
    pub background: Option<Color>,
    pub padding: f64,
}
impl Default for TooltipOptions {
    fn default() -> Self {
        Self {
            lines: vec![vec![
                TooltipPart::Label,
                TooltipPart::Text(": ".into()),
                TooltipPart::Value,
            ]],
            heading: None,
            value_format: ValueFormat::Auto,
            series_labels: BTreeMap::new(),
            missing: MissingValue::HideTooltip,
            max_rows: 32,
            overflow_text: Some("…".into()),
            text: PresentationText::default(),
            background: None,
            padding: 0.,
        }
    }
}
impl TooltipOptions {
    pub fn validate(&self) -> Result<()> {
        self.text.validate()?;
        self.value_format.validate()?;
        labels_budget(&self.series_labels)?;
        ensure(
            !self.lines.is_empty()
                && self.lines.len() <= 16
                && self.lines.iter().all(|l| l.len() <= 32),
            "invalid tooltip template budget",
        )?;
        for part in self.lines.iter().flatten() {
            if let TooltipPart::Text(text) = part {
                text_budget(text)?;
            }
        }
        for text in [&self.heading, &self.overflow_text].into_iter().flatten() {
            text_budget(text)?;
        }
        if let MissingValue::Placeholder(text) = &self.missing {
            text_budget(text)?;
        }
        ensure(
            (1..=256).contains(&self.max_rows),
            "tooltip row limit must be between 1 and 256",
        )?;
        ensure(
            self.padding.is_finite() && (0. ..=4096.).contains(&self.padding),
            "invalid tooltip padding",
        )
    }
}
/// Nullable values can be supplied by application slice data; Scene marks always
/// contain a finite value. Nonfinite Some values are invalid, not missing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipDatum<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub series: &'a str,
    pub value: Option<f64>,
}
impl<'a> From<&'a Mark> for TooltipDatum<'a> {
    fn from(mark: &'a Mark) -> Self {
        Self {
            id: &mark.id,
            label: &mark.label,
            series: &mark.series,
            value: Some(mark.value),
        }
    }
}
pub fn tooltip_text(options: &TooltipOptions, marks: &[&Mark]) -> Result<String> {
    ensure(
        marks.len() <= crate::MAX_MARKS,
        "tooltip input count exceeded",
    )?;
    let data = marks
        .iter()
        .map(|m| TooltipDatum::from(*m))
        .collect::<Vec<_>>();
    tooltip_data_text(options, &data)
}
pub fn tooltip_data_text(options: &TooltipOptions, data: &[TooltipDatum<'_>]) -> Result<String> {
    options.validate()?;
    ensure(
        data.len() <= crate::MAX_MARKS,
        "tooltip input count exceeded",
    )?;
    let mut lines = Vec::new();
    let mut shown = 0usize;
    let mut overflow = false;
    for datum in data {
        for text in [datum.id, datum.label, datum.series] {
            text_budget(text)?;
        }
        ensure(
            datum.value.is_none_or(f64::is_finite),
            "tooltip value must be finite",
        )?;
        let value = match datum.value {
            Some(value) => options.value_format.format(value)?,
            None => match &options.missing {
                MissingValue::HideTooltip => return Ok(String::new()),
                MissingValue::OmitRow => continue,
                MissingValue::Placeholder(text) => text.clone(),
            },
        };
        if shown >= options.max_rows {
            overflow = true;
            continue;
        }
        shown += 1;
        for template in &options.lines {
            let mut line = String::new();
            for part in template {
                line.push_str(match part {
                    TooltipPart::Text(text) => text,
                    TooltipPart::Label => datum.label,
                    TooltipPart::Id => datum.id,
                    TooltipPart::Series => options
                        .series_labels
                        .get(datum.series)
                        .map_or(datum.series, String::as_str),
                    TooltipPart::Value => &value,
                });
            }
            text_budget(&line)?;
            lines.push(line);
        }
    }
    if shown == 0 {
        return Ok(String::new());
    }
    if let Some(heading) = &options.heading {
        lines.insert(0, heading.clone());
    }
    if overflow && let Some(text) = &options.overflow_text {
        lines.push(text.clone());
    }
    let text = lines.join("\n");
    ensure(text.len() <= 16_384, "tooltip total text budget exceeded")?;
    Ok(text)
}
pub fn tooltip_style(options: &TooltipOptions, visible: bool) -> Style {
    Style {
        background: options.background,
        padding: Some(Edges::all(options.padding)),
        ..text_style(&options.text)
            .display(if visible {
                Display::Block
            } else {
                Display::None
            })
            .white_space(WhiteSpace::PreWrap)
    }
}
