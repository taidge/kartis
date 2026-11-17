use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use serde_json::json;

use super::{phase, *};

const WORLD_SOURCE_URL: &str = "https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/website/src/data/components/geo/world_countries.json";
const WORLD_SHA256: &str = "bb261743fc2aa5e09ca70a2823e307bfe084fbb812df066a55fb2bf898248e17";

fn land(seed: u32) -> Result<Vec<GeoFeature>> {
    static WORLD: std::sync::OnceLock<Result<Vec<GeoFeature>>> = std::sync::OnceLock::new();
    let original = WORLD
        .get_or_init(|| {
            geodata::parse_geojson(
                include_str!("../../data/world_countries.closed.geojson"),
                &geodata::GeoParseOptions::default(),
            )
        })
        .as_ref()
        .map_err(Clone::clone)?;
    if original.len() != 176 {
        return Err(ChartError(
            "The bundled world GeoJSON must contain all 176 features".into(),
        ));
    }
    // Keep all source data bundled. The current planar projection deliberately
    // uses the documented non-Antarctic subset; it cannot represent a polar cap.
    let mut features = original
        .iter()
        .filter(|feature| feature.id != "ATA")
        .cloned()
        .collect::<Vec<_>>();
    if features.len() != 175 {
        return Err(ChartError(
            "The world sample must render exactly the 175 non-Antarctic features".into(),
        ));
    }
    for feature in &mut features {
        // Stable country IDs, not collection indices, own the synthetic values.
        let hash = feature.id.bytes().fold(2_166_136_261_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
        });
        feature.value = Some(50. + (phase(seed) + f64::from(hash % 6283) / 1000.).sin() * 35.);
    }
    Ok(features)
}
pub(super) fn sample(kind: &str, c: &ChartConfig, seed: u32) -> Result<Sample> {
    let p = phase(seed);
    let (scene, data, code) = match kind {
        "choropleth" | "geo-map" => {
            let source = land(seed)?;
            let cities = [
                (-74., 41.),
                (-3., 51.),
                (31., 30.),
                (120., 35.),
                (135., -26.),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, pos)| {
                GeoSymbol::new(
                    format!("City {}", i + 1),
                    [pos.0, pos.1],
                    Some(30. + (p + i as f64).sin() * 20.),
                )
            })
            .collect::<Vec<_>>();
            let mut data = json!({"regions":source.iter().map(|f| json!({"id":f.id,"name":f.label,"value":f.value,"polygons":f.polygons})).collect::<Vec<_>>(),"dataset":{"name":"Nivo world countries","sourceFeatureCount":176,"renderedFeatureCount":source.len(),"excluded":[{"id":"ATA","reason":"The current planar projection profile does not accept pole-enclosing spherical-cap topology"}],"normalization":"The derived GeoJSON closes the original KOR open ring by appending its first coordinate; all original coordinates are retained","source":WORLD_SOURCE_URL,"sha256":WORLD_SHA256,"license":"MIT","licenseNotice":"THIRD_PARTY_NOTICES.txt","values":"Synthetic demonstration values assigned by stable country ID; not observed country statistics"}});
            if kind == "choropleth" {
                (
                    choropleth_layout(
                        c,
                        &source,
                        &GeoMapOptions {
                            projection: GeoProjection::Mercator,
                            ..Default::default()
                        },
                    )?
                    .scene,
                    data,
                    "let regions = geodata::parse_geojson(include_str!(\"world_countries.closed.geojson\"), &geodata::GeoParseOptions::default())?;\n// Explicit non-Antarctic subset; the planar profile rejects polar-cap topology.\nlet mut regions: Vec<_> = regions.into_iter().filter(|region| region.id != \"ATA\").collect();\n// Assign synthetic values by stable country ID.\nchoropleth_layout(&config, &regions, &GeoMapOptions { projection: GeoProjection::Mercator, ..Default::default() })?.scene",
                )
            } else {
                data["cities"] = json!(
                    cities
                        .iter()
                        .map(|f| json!({"id":f.id,"coordinates":f.position,"value":f.value}))
                        .collect::<Vec<_>>()
                );
                let mut map = geographic_map_layout(
                    c,
                    &source,
                    &GeoMapOptions {
                        projection: GeoProjection::Equirectangular,
                        fill: Some(Color(220, 235, 231, 255)),
                        ..Default::default()
                    },
                )?;
                map.add_graticule(
                    &GeoGraticuleOptions {
                        step_minor: [30., 30.],
                        ..Default::default()
                    },
                    &GeoStreamOptions {
                        behind: true,
                        ..Default::default()
                    },
                )?;
                map.add_symbols(
                    &cities,
                    &GeoSymbolOptions {
                        radius_range: [4., 14.],
                        ..Default::default()
                    },
                )?;
                (
                    map.scene,
                    data,
                    "let regions = geodata::parse_geojson(include_str!(\"world_countries.closed.geojson\"), &geodata::GeoParseOptions::default())?;\n// Explicit non-Antarctic subset; retain the complete 176-feature source file.\nlet regions: Vec<_> = regions.into_iter().filter(|region| region.id != \"ATA\").collect();\nlet mut map = geographic_map_layout(&config, &regions, &GeoMapOptions { fill: Some(Color(220, 235, 231, 255)), ..Default::default() })?;\nmap.add_graticule(&GeoGraticuleOptions { step_minor: [30., 30.], ..Default::default() }, &GeoStreamOptions { behind: true, ..Default::default() })?;\nmap.add_symbols(&cities, &GeoSymbolOptions { radius_range: [4., 14.], ..Default::default() })?;\nlet scene = map.scene;",
                )
            }
        }
        "voronoi" | "delaunay" => {
            let source = (0..24)
                .map(|i| {
                    let angle = f64::from(i) * 2.399963229728653 + p * 0.08;
                    let r = (f64::from(i + 1) / 24.).sqrt();
                    (
                        format!("site-{i}"),
                        (angle.cos() * r * 0.42 + 0.5),
                        (angle.sin() * r * 0.42 + 0.5),
                    )
                })
                .collect::<Vec<_>>();
            let data = json!({"sites":source.iter().map(|(id,x,y)|json!({"id":id,"x":x,"y":y})).collect::<Vec<_>>()});
            if kind == "delaunay" {
                (
                    Delaunay::new(
                        source
                            .iter()
                            .map(|(id, x, y)| DelaunaySite::new(id, *x, *y))
                            .collect(),
                    )?
                    .diagram(
                        c,
                        &DelaunayDiagramOptions {
                            triangle_fill: Some(Color(34, 126, 119, 35)),
                            site_color: c.theme.color(1),
                            link_color: c.theme.color(0),
                            labels: false,
                            ..Default::default()
                        },
                    )?
                    .scene,
                    data,
                    "Delaunay::new(sites)?.diagram(&config, &DelaunayDiagramOptions::default())?.scene",
                )
            } else {
                let plot = ClipRect::plot(c);
                let mesh = mesh::PointMesh::new(
                    source
                        .iter()
                        .map(|(id, x, y)| mesh::MeshPoint {
                            id: id.clone(),
                            x: plot.x + x * plot.width,
                            y: plot.y + y * plot.height,
                            clip: None,
                        })
                        .collect(),
                )?;
                let mut scene = c.scene()?;
                for (i, cell) in mesh.voronoi(plot)?.into_iter().enumerate() {
                    let mut path = cell
                        .polygon
                        .into_iter()
                        .enumerate()
                        .map(|(i, (x, y))| {
                            if i == 0 {
                                PathCommand::Move(GraphicPoint::new(x, y))
                            } else {
                                PathCommand::Line(GraphicPoint::new(x, y))
                            }
                        })
                        .collect::<Vec<_>>();
                    path.push(PathCommand::Close);
                    let mut mark = Mark::new(
                        format!("cell:{}", cell.id),
                        Geometry::Path(path),
                        c.theme.color(i % 5),
                    );
                    mark.series = cell.id.clone();
                    mark.label = cell.id;
                    mark.opacity = 0.45;
                    mark.stroke = Some(GraphicStroke {
                        color: Color(255, 255, 255, 255),
                        width: 1.5,
                        cap: StrokeCap::Butt,
                    });
                    scene.marks.push(mark);
                }
                for point in mesh.points() {
                    scene.marks.push(Mark::new(
                        format!("point:{}", point.id),
                        Geometry::Circle {
                            x: point.x,
                            y: point.y,
                            radius: 2.8,
                        },
                        Color(55, 77, 80, 255),
                    ));
                }
                (
                    scene,
                    data,
                    "let mesh = mesh::PointMesh::new(sites)?;\nlet cells = mesh.voronoi(ClipRect::plot(&config))?; // Render each returned polygon",
                )
            }
        }
        "contour" => {
            let values = (0..32 * 24)
                .map(|i| {
                    let x = (i % 32) as f64 / 31.;
                    let y = (i / 32) as f64 / 23.;
                    Some(
                        (-((x - 0.35 - p.sin() * 0.08).powi(2) + (y - 0.45).powi(2)) * 18.).exp()
                            * 80.
                            + (-((x - 0.73).powi(2) + (y - 0.63 - p.cos() * 0.08).powi(2)) * 30.)
                                .exp()
                                * 60.,
                    )
                })
                .collect::<Vec<_>>();
            let data =
                json!({"width":32,"height":24,"values":values,"thresholds":[10,20,30,40,50,60,70]});
            (
                contour_layout(
                    c,
                    ContourGrid {
                        width: 32,
                        height: 24,
                        values: &values,
                    },
                    &ContourOptions {
                        thresholds: ContourThresholds::Values(vec![
                            10., 20., 30., 40., 50., 60., 70.,
                        ]),
                        ..Default::default()
                    },
                    &ContourLayoutOptions {
                        colors: ColorScaleOptions {
                            colors: vec![Color(217, 239, 226, 255), Color(19, 119, 122, 255)],
                            domain: Some([0., 80.]),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?
                .scene,
                data,
                "contour_layout(&config, grid, &ContourOptions::default(), &ContourLayoutOptions::default())?.scene",
            )
        }
        "density" => {
            let plot = ClipRect::plot(c);
            let source = (0..100)
                .map(|i| {
                    let angle = f64::from(i) * 2.399963229728653;
                    let radius = (f64::from(i % 50 + 1) / 50.).sqrt();
                    let group = if i < 50 { 0.34 } else { 0.7 };
                    let x = (group + radius * 0.17 * angle.cos() + p.sin() * 0.03) * plot.width;
                    let y = (0.5 + radius * 0.29 * angle.sin() + p.cos() * 0.04) * plot.height;
                    DensityPoint {
                        position: [x, y],
                        weight: 1.,
                    }
                })
                .collect::<Vec<_>>();
            let data = json!({"observations":source.iter().map(|point|json!({"position":point.position,"weight":point.weight})).collect::<Vec<_>>(),"bandwidth":16,"cellSize":4});
            let field = estimate_density(
                &source,
                DensityOptions {
                    size: [plot.width, plot.height],
                    bandwidth: 16.,
                    cell_size: 4.,
                    ..Default::default()
                },
            )?;
            (
                density_layout(
                    c,
                    &field,
                    &DensityThresholds::Count(7),
                    ContourLimits::default(),
                    &ContourLayoutOptions {
                        colors: ColorScaleOptions {
                            colors: vec![Color(240, 229, 214, 255), Color(147, 106, 170, 255)],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?
                .scene,
                data,
                "let field = estimate_density(&points, DensityOptions { size: plot_size, ..Default::default() })?;\ndensity_layout(&config, &field, &DensityThresholds::Count(7), ContourLimits::default(), &ContourLayoutOptions::default())?.scene",
            )
        }
        _ => return Err(ChartError(format!("Unknown spatial chart: {kind}"))),
    };
    Ok(Sample { scene, data, code })
}
