#[cfg(feature = "raster")]
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use kartis::render::raster::*;
    use kartis::*;
    let destination = std::env::args()
        .nth(1)
        .ok_or("provide an output PNG path")?;
    let mut fonts = FontRegistry::new();
    fonts.register(
        include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    let config = ChartConfig {
        width: 720.,
        height: 400.,
        margin: Margin {
            bottom: 86.,
            ..Default::default()
        },
        title: "CPU-rendered chart".into(),
        ..Default::default()
    };
    let scene = bar(
        &config,
        &[
            "January".into(),
            "February".into(),
            "March".into(),
            "April".into(),
        ],
        &[
            BarSeries::new("Revenue", vec![Some(30.), Some(48.), Some(42.), Some(67.)]),
            BarSeries::new("Target", vec![Some(40.), Some(40.), Some(50.), Some(60.)]),
        ],
        &BarOptions {
            category_axis: CategoryAxisOptions {
                tick_rotation: 25.,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let image = render_raster(
        &scene,
        &fonts,
        &RasterOptions {
            pixel_ratio: 2.,
            ..Default::default()
        },
    )?;
    std::fs::write(&destination, image.png()?)?;
    println!(
        "CPU raster: {}x{} physical pixels -> {destination}",
        image.width, image.height
    );
    Ok(())
}
#[cfg(not(feature = "raster"))]
fn main() {
    eprintln!("Run with --features raster and an output PNG path.");
}
