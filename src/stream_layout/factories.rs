//! Borrowed typed counterparts of Stream's eight function-valued style roles.
use super::*;

/// Original layer ID and area geometry, before applying label/color callbacks.
/// Geometry uses full Scene pixels; `None` means the layer has no samples/path.
/// This is a borrowed Rust Geometry, not an SVG path string.
#[derive(Clone, Copy, Debug)]
pub struct StreamLayerPathContext<'a> {
    pub series: &'a BarSeries,
    pub series_index: usize,
    pub id: &'a str,
    pub path: Option<&'a Geometry>,
    pub plot: ClipRect,
}

/// Complete current layer, with all formatted samples, resolved label and color.
/// The actual visible area ID is populated before the border callback.
#[derive(Clone, Copy, Debug)]
pub struct StreamLayerStyleContext<'a> {
    pub series: &'a BarSeries,
    pub series_index: usize,
    pub layer: &'a StreamLayer,
    pub path: Option<&'a Geometry>,
    pub plot: ClipRect,
}

/// One actual non-null dot with the complete computed layer, not a prefix.
/// `style` is the declared default configuration. An explicit callback replaces
/// its corresponding default without resolving an unused inherited color.
#[derive(Clone, Copy, Debug)]
pub struct StreamDotStyleContext<'a> {
    pub series: &'a BarSeries,
    pub series_index: usize,
    pub layer: &'a StreamLayer,
    pub sample: &'a StreamSample,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub style: &'a StreamDotOptions,
}

pub type StreamLayerLabelFactory<'a> =
    dyn FnMut(&StreamLayerPathContext<'_>) -> Result<String> + 'a;
pub type StreamLayerColorFactory<'a> = dyn FnMut(&StreamLayerPathContext<'_>) -> Result<Color> + 'a;
pub type StreamValueFormatter<'a> = dyn FnMut(f64) -> Result<String> + 'a;
pub type StreamLayerBorderColorFactory<'a> =
    dyn FnMut(&StreamLayerStyleContext<'_>) -> Result<Color> + 'a;
pub type StreamDotSizeFactory<'a> = dyn FnMut(&StreamDotStyleContext<'_>) -> Result<f64> + 'a;
pub type StreamDotColorFactory<'a> = dyn FnMut(&StreamDotStyleContext<'_>) -> Result<Color> + 'a;

/// Eight synchronous functions borrowed for one build, never retained.
/// Label/color/formatter run for hidden layers; visible area border callbacks
/// run even at zero border width. Dot functions run only for enabled, visible
/// Some samples, including Some(0) and size zero. Rust None stays nullable.
/// Explicit empty labels are kept. Numeric/text outputs are checked; errors
/// return no layout, while callback-owned external side effects cannot roll back.
#[derive(Default)]
pub struct StreamFactories<'a> {
    pub label: Option<&'a mut StreamLayerLabelFactory<'a>>,
    pub color: Option<&'a mut StreamLayerColorFactory<'a>>,
    pub formatted_value: Option<&'a mut StreamValueFormatter<'a>>,
    pub border_color: Option<&'a mut StreamLayerBorderColorFactory<'a>>,
    pub dot_size: Option<&'a mut StreamDotSizeFactory<'a>>,
    pub dot_color: Option<&'a mut StreamDotColorFactory<'a>>,
    pub dot_border_width: Option<&'a mut StreamDotSizeFactory<'a>>,
    pub dot_border_color: Option<&'a mut StreamDotColorFactory<'a>>,
}

pub fn stream_layout_with_factories(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
    factories: &mut StreamFactories<'_>,
) -> Result<StreamLayout> {
    layout(c, categories, series, o, factories, None)
}

/// Resolve function styles before the existing custom dot geometry callback.
/// `StreamDotContext.fill` and `.stroke` describe the resolved paint; `.style`
/// remains the caller's declared defaults. `inherit` keeps resolved paint.
/// A dot-size function together with any configured glyph is rejected before
/// callbacks because circle diameter and glyph area/nominal size differ.
pub fn stream_layout_with_factories_and_dot_factory(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
    factories: &mut StreamFactories<'_>,
    mut factory: impl FnMut(&StreamDotContext<'_>) -> Result<StreamDotGeometry>,
) -> Result<StreamLayout> {
    layout(c, categories, series, o, factories, Some(&mut factory))
}
