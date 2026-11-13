//! Spherical measurements feed actual map geometry and visible unit annotations.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
fn frame(c: &ChartConfig) -> ChartConfig {
    ChartConfig {
        margin: Margin {
            left: 36.,
            right: 28.,
            top: 24.,
            bottom: 100.,
        },
        ..c.clone()
    }
}
fn note(scene: &mut Scene, row: usize, text: String) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: scene.width / 2.,
        y: scene.height - 78. + row as f64 * 24.,
        text,
        color: Color(45, 55, 70, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn line_style() -> GeoStreamOptions {
    GeoStreamOptions {
        interactive: false,
        style: GeoLineStyle {
            stroke: GraphicStroke {
                color: Color(145, 75, 135, 255),
                width: 2.,
                cap: StrokeCap::Round,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
pub fn circle_scene(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let center = [15., 25.];
    let ring = geo_circle(&GeoCircleOptions {
        center,
        radius_degrees: 25.,
        precision_degrees: 3.,
        ..Default::default()
    })?;
    let area = geo_spherical_area(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
    let perimeter = geo_spherical_length(GeoSphericalGeometry::LineString(&ring))?;
    let streamed =
        geo_spherical_length(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
    let centroid =
        geo_spherical_centroid(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?
            .ok_or_else(|| ChartError("consumer polygon has no centroid".into()))?;
    let contains = geo_spherical_contains(
        GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)),
        [15., 25.],
    )?;
    let feature = GeoFeature {
        id: "spherical-circle".into(),
        label: format!("Sampled circle area: {area:.6} sr"),
        value: Some(area),
        polygons: vec![vec![ring.clone()]],
    };
    let view = GeoMapOptions {
        projection: GeoProjection::Orthographic,
        center,
        fit: GeoFit::World,
        scale: Some(220.),
        translate: Some([c.width / 2., c.height * 0.43]),
        fill: Some(Color(36, 126, 144, 255)),
        border: None,
        ..Default::default()
    };
    let mut map = geographic_map_layout(&c, &[feature], &view)?;
    let mut boundary = GeoLineFeature::new("circle-perimeter", ring.clone());
    boundary.value = Some(perimeter);
    map.add_lines(&[boundary], &line_style())?;
    let mut symbols = vec![GeoSymbol::new(
        "circle-centre",
        center,
        Some(geo_spherical_distance(center, center)?),
    )];
    for (i, index) in [
        0,
        (ring.len() - 1) / 4,
        (ring.len() - 1) / 2,
        3 * (ring.len() - 1) / 4,
    ]
    .into_iter()
    .enumerate()
    {
        let p = ring[index];
        let value = geo_spherical_distance(center, p)?;
        let mut symbol = GeoSymbol::new(format!("circle-boundary-{i}"), p, Some(value));
        symbol.label = format!("Boundary distance: {value:.6} rad");
        symbols.push(symbol);
    }
    map.add_symbols(
        &symbols,
        &GeoSymbolOptions {
            radius_range: [4., 4.],
            zero_radius: 4.,
            color: GeoSymbolColor::Fixed(Color(220, 105, 35, 255)),
            ..Default::default()
        },
    )?;
    note(
        &mut map.scene,
        0,
        "Center [15,25] deg | radius 25 deg | sampling step 3 deg".into(),
    );
    note(
        &mut map.scene,
        1,
        format!("Area {area:.6} sr | closed LineString perimeter {perimeter:.6} rad"),
    );
    note(
        &mut map.scene,
        2,
        format!("D3 Polygon streamed length {streamed:.6} rad (closing edge omitted)"),
    );
    note(
        &mut map.scene,
        3,
        format!(
            "Centroid [{:.6},{:.6}] deg | contains [15,25] deg: {contains}",
            centroid[0], centroid[1]
        ),
    );
    map.scene.validate()?;
    Ok(map.scene)
}
pub fn triangle_scene(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let ring = vec![[0., 0.], [0., 90.], [90., 0.], [0., 0.]];
    let mut reversed = ring.clone();
    reversed.reverse();
    let area = geo_spherical_area(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
    let complement = geo_spherical_area(GeoSphericalGeometry::Polygon(&[reversed]))?;
    let perimeter = geo_spherical_length(GeoSphericalGeometry::LineString(&ring))?;
    let streamed =
        geo_spherical_length(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
    let d = geo_spherical_distance(ring[0], ring[2])?;
    let centroid =
        geo_spherical_centroid(GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?
            .ok_or_else(|| ChartError("consumer polygon has no centroid".into()))?;
    let contains = geo_spherical_contains(
        GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)),
        [30., 30.],
    )?;
    let feature = GeoFeature {
        id: "spherical-octant".into(),
        label: format!("Octant area: {area:.6} sr"),
        value: Some(area),
        polygons: vec![vec![ring.clone()]],
    };
    let view = GeoMapOptions {
        projection: GeoProjection::Orthographic,
        center: [45., 30.],
        fit: GeoFit::World,
        scale: Some(140.),
        translate: Some([c.width / 2., c.height * 0.43]),
        fill: Some(Color(89, 114, 192, 255)),
        border: None,
        ..Default::default()
    };
    let mut map = geographic_map_layout(&c, &[feature], &view)?;
    let mut boundary = GeoLineFeature::new("octant-perimeter", ring.clone());
    boundary.value = Some(perimeter);
    map.add_lines(&[boundary], &line_style())?;
    let mut symbols = Vec::new();
    for (id, p) in ["A", "B", "C"].into_iter().zip(&ring[..3]) {
        let distance = geo_spherical_distance(ring[0], *p)?;
        let mut symbol = GeoSymbol::new(format!("vertex-{id}"), *p, Some(distance));
        symbol.label = format!("Distance from A: {distance:.6} rad");
        symbols.push(symbol);
    }
    map.add_symbols(
        &symbols,
        &GeoSymbolOptions {
            radius_range: [4., 4.],
            zero_radius: 4.,
            color: GeoSymbolColor::Fixed(Color(220, 105, 35, 255)),
            ..Default::default()
        },
    )?;
    note(
        &mut map.scene,
        0,
        format!("A [0,0], B [0,90], C [90,0] deg | distance A-C {d:.6} rad"),
    );
    note(
        &mut map.scene,
        1,
        format!("Clockwise area {area:.6} sr | reversed complement {complement:.6} sr"),
    );
    note(
        &mut map.scene,
        2,
        format!("Polygon streamed {streamed:.6} rad | closed LineString {perimeter:.6} rad"),
    );
    note(
        &mut map.scene,
        3,
        format!(
            "Centroid [{:.6},{:.6}] deg | contains [30,30] deg: {contains}",
            centroid[0], centroid[1]
        ),
    );
    map.scene.validate()?;
    Ok(map.scene)
}
pub fn gallery_scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    Ok(vec![
        ("Spherical math · sampled small circle", circle_scene(c)?),
        (
            "Spherical math · distance and oriented area",
            triangle_scene(c)?,
        ),
    ])
}
