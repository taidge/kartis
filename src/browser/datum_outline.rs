//! A small, passive SVG above the chart keeps pointer feedback independent of
//! the underlying SVG tree or Canvas backing store.
use super::*;

pub(super) struct DatumOutline {
    layer: HtmlElement,
    svg: svg::RetainedSvg,
    scene: Scene,
    last_view: Option<ZoomTransform>,
}

impl DatumOutline {
    pub(super) fn new(document: &web_sys::Document) -> Result<Self> {
        let layer = element(document, "kartis-datum-outline")?;
        layer.set_attribute("aria-hidden", "true").map_err(host)?;
        for (name, value) in [
            ("position", "absolute"),
            ("inset", "0"),
            ("pointer-events", "none"),
            ("z-index", "1"),
        ] {
            css(&layer, name, value)?;
        }
        let mut scene = ChartConfig::default().scene()?;
        scene.background = Color(0, 0, 0, 0);
        Ok(Self {
            layer,
            svg: svg::RetainedSvg::new(document)?,
            scene,
            last_view: None,
        })
    }

    pub(super) fn paint(
        &mut self,
        surface: &HtmlElement,
        model: &SurfaceModel,
        ids: &[String],
    ) -> Result<()> {
        let source = model.scene();
        let mut marks = Vec::new();
        for id in ids
            .iter()
            .map(String::as_str)
            .chain(model.selected().filter(|_| ids.is_empty()))
        {
            let Some(mark) = model.datum(id) else {
                continue;
            };
            if model.widget_replacements().contains(id)
                || matches!(mark.geometry, Geometry::Circle { radius: 0., .. })
            {
                continue;
            }
            let ink = mark
                .fill
                .or_else(|| mark.stroke.map(|stroke| stroke.color))
                .unwrap_or(source.background);
            let luminance =
                0.2126 * f64::from(ink.0) + 0.7152 * f64::from(ink.1) + 0.0722 * f64::from(ink.2);
            let color = if luminance < 128. {
                Color(250, 252, 255, 255)
            } else {
                Color(28, 43, 65, 255)
            };
            let mut outline = Mark::new(id, mark.geometry.clone(), color);
            outline.fill = None;
            outline.stroke = Some(glory::render::GraphicStroke {
                color,
                width: 2.,
                cap: glory::render::StrokeCap::Round,
            });
            outline.clip = mark.clip;
            outline.clip_path = mark.clip_path.clone();
            outline.opacity = mark.opacity;
            marks.push(outline);
        }
        if marks.is_empty() {
            if self.layer.parent_node().is_some() {
                self.layer.remove();
            }
            return Ok(());
        }
        let view = model.paint_transform();
        if self.last_view != Some(view)
            || self.scene.width != source.width
            || self.scene.height != source.height
            || self.scene.marks != marks
        {
            self.last_view = None;
            self.scene.width = source.width;
            self.scene.height = source.height;
            self.scene.marks = marks;
            self.svg.paint(&self.layer, &self.scene, view)?;
            self.last_view = Some(view);
        }
        if self.layer.parent_node().is_none() {
            surface.append_child(&self.layer).map_err(host)?;
        }
        Ok(())
    }
}
