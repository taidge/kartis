#![allow(dead_code)]
use kartis::render::raster::*;
use kartis::*;
pub(super) fn export(name: &str, scene: &Scene, image: &RenderedImage) {
    if let Some(directory) = std::env::var_os("KARTIS_CONTOUR_ARTIFACTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(format!("{name}.png")), image.png().unwrap()).unwrap();
        std::fs::write(
            directory.join(format!("{name}.svg")),
            render::to_svg(scene).unwrap(),
        )
        .unwrap();
        let svg = render::to_svg(scene).unwrap();
        let canvas = render::to_canvas_script(scene, "canvas").unwrap();
        std::fs::write(directory.join(format!("{name}.html")),format!("<!doctype html><meta charset=utf-8><style>body{{margin:0}}svg,canvas{{display:inline-block}}</style>{svg}<canvas id=canvas></canvas><script>{canvas}</script>")).unwrap();
    }
}
pub(super) fn pixel(image: &RenderedImage, x: usize, y: usize) -> &[u8] {
    let offset = (y * image.width as usize + x) * 4;
    &image.rgba[offset..offset + 4]
}
