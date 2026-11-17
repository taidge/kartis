//! Typed component targets for real Scene marks and labels. Slot identity is
//! independent of a label's current ordinal; no synthetic Mark is introduced.
use std::collections::{BTreeMap, BTreeSet};

use glory::Bond;

use crate::{
    ClipPath, ClipRect, Label, LabelWhitespace, LayoutRect, PlotWidgetFrame, PlotWidgetPlacement,
    ResolvedFont, TextSize,
};

/// A real item in the candidate Scene. Label ordinals are frame-local references,
/// while `SceneWidgetSlot::key` determines retained component identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SceneWidgetTarget {
    Mark(String),
    Label(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneWidgetSlot {
    pub key: String,
    pub target: SceneWidgetTarget,
    pub revision: u64,
    /// Rect is an explicit axis-aligned placement. AtDatum uses the mark's
    /// admitted anchor, or the label's actual `(x, y)` anchor. For a label its
    /// offset and measured box rotate together with the source label.
    pub placement: PlotWidgetPlacement,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneWidgetsLayout {
    pub plot: Option<ClipRect>,
    pub slots: Vec<SceneWidgetSlot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneWidgetsData<T: Clone + PartialEq + 'static> {
    pub layout: SceneWidgetsLayout,
    pub data: BTreeMap<String, T>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneWidgetsComposition<T: Clone + PartialEq + 'static> {
    pub widgets: SceneWidgetsData<T>,
    /// Slot keys whose corresponding real source item's ink is replaced.
    /// A label replacement leaves all node/link Marks and business bindings.
    pub replace_ink: BTreeSet<String>,
}

/// Local measured component coordinates mapped into the rendered Scene:
/// `(x, y) -> (a*x+c*y+e, b*x+d*y+f)` for `[a,b,c,d,e,f]`.
/// The box is laid out before applying this affine transform to its subtree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneWidgetTransform {
    pub matrix: [f64; 6],
    pub measured: TextSize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneLabelWidgetFrame {
    pub label_index: usize,
    pub label: Label,
    pub owner: Option<String>,
    pub font: ResolvedFont,
    pub whitespace: LabelWhitespace,
    pub clip: Option<ClipPath>,
    /// Rendered axis-aligned envelope, including an inherited label rotation.
    pub rect: LayoutRect,
    pub visible: Option<LayoutRect>,
    /// Fully admitted content may receive keyboard focus. Label wrappers use
    /// pointer-events:none by default; a custom child can explicitly opt in.
    /// This is distinct from adding built-in datum event handlers.
    pub interactive: bool,
    pub transform: Option<SceneWidgetTransform>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneWidgetFrame {
    Mark(PlotWidgetFrame),
    Label(SceneLabelWidgetFrame),
}
impl SceneWidgetFrame {
    pub fn rect(&self) -> LayoutRect {
        match self {
            Self::Mark(frame) => frame.rect,
            Self::Label(frame) => frame.rect,
        }
    }
    pub fn visible(&self) -> Option<LayoutRect> {
        match self {
            Self::Mark(frame) => frame.visible,
            Self::Label(frame) => frame.visible,
        }
    }
    pub fn interactive(&self) -> bool {
        match self {
            Self::Mark(frame) => frame.interactive,
            Self::Label(frame) => frame.interactive,
        }
    }
    pub fn is_label(&self) -> bool {
        matches!(self, Self::Label(_))
    }
    pub fn transform(&self) -> Option<SceneWidgetTransform> {
        match self {
            Self::Mark(_) => None,
            Self::Label(frame) => frame.transform,
        }
    }
    pub(crate) fn mark_frame(&self) -> crate::Result<PlotWidgetFrame> {
        match self {
            Self::Mark(frame) => Ok(frame.clone()),
            Self::Label(_) => Err(crate::ChartError(
                "Mark component received a label target".into(),
            )),
        }
    }
}

#[derive(Clone)]
pub struct SceneWidgetContext {
    pub key: String,
    pub revision: u64,
    pub frame: Bond<SceneWidgetFrame>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedSceneWidgetFrame<T: Clone + PartialEq + 'static> {
    pub frame: SceneWidgetFrame,
    pub data: T,
}

#[derive(Clone)]
pub struct SceneWidgetCompositionContext<T: Clone + PartialEq + 'static> {
    pub key: String,
    pub revision: u64,
    pub frame: Bond<TypedSceneWidgetFrame<T>>,
}
