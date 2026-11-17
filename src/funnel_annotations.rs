//! Source-bound Funnel annotations, using the SVG annotation composition order.
//! Factories are borrowed for a single transactional build and never retained.
//! This static Scene projection does not reproduce React components or springs.
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;
mod bind;
mod paint;

#[derive(Clone, Debug, PartialEq)]
pub enum FunnelAnnotationMatch {
    All,
    Id(CategoryKey),
    Predicate(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunnelAnnotationKind {
    Circle,
    Dot,
    Rect,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunnelAnnotationPosition {
    Relative(f64),
    Absolute(f64),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunnelAnnotationNote {
    Text(String),
    Custom(String),
}
/// Owned annotation recipe. Source getters deliberately override x/y and rect
/// width/height, matching the upstream binder's spread order. Only size uses
/// truthiness: an explicit zero falls back to the derived dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelAnnotationSpec {
    pub matcher: FunnelAnnotationMatch,
    pub kind: FunnelAnnotationKind,
    pub note: FunnelAnnotationNote,
    pub note_x: FunnelAnnotationPosition,
    pub note_y: FunnelAnnotationPosition,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub size: Option<f64>,
    pub offset: f64,
    pub note_width: f64,
    pub note_text_offset: f64,
    pub border_radius: f64,
}
impl FunnelAnnotationSpec {
    pub fn new(
        matcher: FunnelAnnotationMatch,
        kind: FunnelAnnotationKind,
        note: FunnelAnnotationNote,
        note_x: FunnelAnnotationPosition,
        note_y: FunnelAnnotationPosition,
    ) -> Self {
        Self {
            matcher,
            kind,
            note,
            note_x,
            note_y,
            x: None,
            y: None,
            width: None,
            height: None,
            size: None,
            offset: 0.,
            note_width: 120.,
            note_text_offset: 8.,
            border_radius: 6.,
        }
    }
}
/// Shape/link styles are independent: an annotation need not inherit a Funnel
/// body's paint. The SVG dot halo uses `outline`, while its main dot uses symbol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FunnelAnnotationPaint {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub outline_width: f64,
    pub outline_color: Color,
    pub outline_opacity: f64,
}
impl Default for FunnelAnnotationPaint {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: Some(Color(0, 0, 0, 255)),
            stroke_width: 2.,
            opacity: 1.,
            outline_width: 2.,
            outline_color: Color(255, 255, 255, 255),
            outline_opacity: 1.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelAnnotationTextStyle {
    pub color: Color,
    pub font: FontOptions,
    pub font_size: f64,
    pub outline_width: f64,
    pub outline_color: Color,
}
impl Default for FunnelAnnotationTextStyle {
    fn default() -> Self {
        Self {
            color: Color(51, 51, 51, 255),
            font: FontOptions::default(),
            font_size: 13.,
            outline_width: 2.,
            outline_color: Color(255, 255, 255, 255),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelAnnotationTheme {
    pub outline: FunnelAnnotationPaint,
    pub symbol: FunnelAnnotationPaint,
    pub link: FunnelAnnotationPaint,
    pub text: FunnelAnnotationTextStyle,
}
pub type FunnelAnnotationStyle = FunnelAnnotationTheme;
impl Default for FunnelAnnotationTheme {
    fn default() -> Self {
        Self {
            outline: Default::default(),
            symbol: FunnelAnnotationPaint {
                fill: Some(Color(0, 0, 0, 255)),
                stroke: None,
                stroke_width: 0.,
                ..Default::default()
            },
            link: FunnelAnnotationPaint {
                stroke_width: 1.,
                ..Default::default()
            },
            text: Default::default(),
        }
    }
}
impl FunnelAnnotationTheme {
    pub fn from_chart(theme: &Theme) -> Self {
        Self {
            text: FunnelAnnotationTextStyle {
                color: theme.text,
                font: theme.font.clone(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FunnelAnnotationBudget {
    pub specs: usize,
    pub bindings: usize,
    pub match_work: usize,
    pub marks: usize,
    pub labels: usize,
    pub path_commands: usize,
}
impl Default for FunnelAnnotationBudget {
    fn default() -> Self {
        Self {
            specs: 1024,
            bindings: 1024,
            match_work: 200_000,
            marks: MAX_MARKS,
            labels: MAX_MARKS,
            path_commands: 200_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelAnnotationInstructions {
    pub points: [[f64; 2]; 3],
    pub text: [f64; 2],
    /// Normalized source-to-note angle plus90. It is metadata, not text rotation.
    pub angle: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoundFunnelAnnotation {
    pub spec_index: usize,
    pub part_index: usize,
    pub source_index: usize,
    pub id: CategoryKey,
    pub kind: FunnelAnnotationKind,
    pub note: FunnelAnnotationNote,
    /// Full Scene coordinates, including the chart margin exactly once.
    pub center: [f64; 2],
    pub size: f64,
    pub width: f64,
    pub height: f64,
    pub border_radius: f64,
    pub note_width: f64,
    pub note_text_offset: f64,
    pub computed: FunnelAnnotationInstructions,
}
#[derive(Clone, Copy)]
pub struct FunnelAnnotationMatchContext<'a> {
    pub key: &'a str,
    pub spec_index: usize,
    pub spec: &'a FunnelAnnotationSpec,
    pub frame: &'a FunnelSvgFrame,
    pub part_index: usize,
    pub source_index: usize,
    pub part: &'a FunnelPart,
    pub collection: &'a [FunnelPart],
}
#[derive(Clone, Copy)]
pub struct FunnelAnnotationNoteContext<'a> {
    pub key: &'a str,
    pub frame: &'a FunnelSvgFrame,
    pub binding: &'a BoundFunnelAnnotation,
    pub part: &'a FunnelPart,
    pub datum: &'a FunnelDatum,
    pub x: f64,
    pub y: f64,
}
impl FunnelAnnotationNoteContext<'_> {
    /// A legal transparent, full-size note Scene. Put real custom geometry/text
    /// at x/y (already full Scene coordinates); no implicit translation follows.
    pub fn scene(&self) -> Result<Scene> {
        let mut s = self.frame.config().scene()?;
        s.background = Color(0, 0, 0, 0);
        Ok(s)
    }
}
pub type FunnelAnnotationMatcher<'a> =
    dyn FnMut(FunnelAnnotationMatchContext<'_>) -> Result<bool> + 'a;
pub type FunnelAnnotationNoteRenderer<'a> =
    dyn FnMut(FunnelAnnotationNoteContext<'_>) -> Result<Scene> + 'a;
#[derive(Default)]
pub struct FunnelAnnotationFactories<'a> {
    pub matcher: Option<&'a mut FunnelAnnotationMatcher<'a>>,
    pub note: Option<&'a mut FunnelAnnotationNoteRenderer<'a>>,
}
#[derive(Debug)]
struct AnnotationFrame {
    source: FunnelSvgFrame,
    specs: Vec<FunnelAnnotationSpec>,
    theme: FunnelAnnotationTheme,
    bindings: Vec<BoundFunnelAnnotation>,
    scene: Scene,
}
/// One immutable source/binding/paint cohort. Equality is capture identity;
/// raw signed zero and source changes cannot be hidden by legacy layout Eq.
#[derive(Clone, Debug)]
pub struct FunnelAnnotations(Arc<AnnotationFrame>);
impl PartialEq for FunnelAnnotations {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FunnelAnnotations {}
impl FunnelAnnotations {
    pub fn source(&self) -> &FunnelSvgFrame {
        &self.0.source
    }
    pub fn specs(&self) -> &[FunnelAnnotationSpec] {
        &self.0.specs
    }
    pub fn theme(&self) -> &FunnelAnnotationTheme {
        &self.0.theme
    }
    pub fn bindings(&self) -> &[BoundFunnelAnnotation] {
        &self.0.bindings
    }
    /// Transparent independent annotation paint, without canonical business
    /// sites. Order follows SVG annotations, including real note text strokes.
    pub fn scene(&self) -> &Scene {
        &self.0.scene
    }
}
pub fn funnel_annotations(
    frame: &FunnelSvgFrame,
    specs: &[FunnelAnnotationSpec],
    factories: &mut FunnelAnnotationFactories<'_>,
    theme: &FunnelAnnotationTheme,
) -> Result<FunnelAnnotations> {
    funnel_annotations_with_budget(
        frame,
        specs,
        factories,
        theme,
        FunnelAnnotationBudget::default(),
    )
}
pub fn funnel_annotations_with_budget(
    frame: &FunnelSvgFrame,
    specs: &[FunnelAnnotationSpec],
    factories: &mut FunnelAnnotationFactories<'_>,
    theme: &FunnelAnnotationTheme,
    budget: FunnelAnnotationBudget,
) -> Result<FunnelAnnotations> {
    let frame = frame.annotation_source();
    bind::preflight(frame, specs, factories, theme, budget)?;
    let bindings = bind::bind(frame, specs, factories, budget)?;
    let scene = paint::paint(frame, &bindings, factories, theme, budget)?;
    Ok(FunnelAnnotations(Arc::new(AnnotationFrame {
        source: frame.clone(),
        specs: specs.to_vec(),
        theme: theme.clone(),
        bindings,
        scene,
    })))
}
