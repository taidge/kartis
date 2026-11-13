//! Deterministic, robust Delaunay adjacency with explicit duplicate ownership.
//!
//! Spade 2.15 performs the triangulation with adaptive exact predicates. We sort
//! input by ID and apply only an exactly reversible uniform power-of-two scale;
//! no point is jittered or snapped. The supported exponent span is bounded by
//! Spade's predicate profile (2^-142 ..= 2^201). Wider dynamic ranges are rejected.
//! Co-circular inputs have more than one valid triangulation: the sorted input
//! and pinned backend choose a deterministic one, not necessarily D3's diagonal.
use std::collections::BTreeMap;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use spade::{DelaunayTriangulation, Point2, Triangulation};

use crate::scene::ensure;
use crate::{ChartConfig, Color, Geometry, Label, LegendItem, Mark, Result, Scene, TextAnchor};

/// Maximum raw sites, including duplicates, in one triangulation.
pub const MAX_DELAUNAY_SITES: usize = 20_000;

#[derive(Clone, Debug, PartialEq)]
pub struct DelaunaySite {
    pub id: String,
    pub x: f64,
    pub y: f64,
}
impl DelaunaySite {
    pub fn new(id: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            id: id.into(),
            x,
            y,
        }
    }
}

/// All indices address `sites()`, except `raw_owners()` which is parallel to
/// `raw_sites()`. Both lists are sorted by ID. Duplicate coordinates (including
/// signed zero) belong to the lexicographically smallest ID at that location.
#[derive(Clone, Debug, PartialEq)]
pub struct Delaunay {
    raw: Vec<DelaunaySite>,
    sites: Vec<DelaunaySite>,
    owners: Vec<usize>,
    normalized: Vec<[f64; 2]>,
    scale_exponent: i32,
    triangles: Vec<usize>,
    halfedges: Vec<Option<usize>>,
    inedges: Vec<Option<usize>>,
    hull: Vec<usize>,
    links: Vec<[usize; 2]>,
    adjacent: Vec<Vec<usize>>,
}

impl Delaunay {
    /// Builds an immutable candidate. Failure cannot mutate an accepted graph.
    pub fn new(mut raw: Vec<DelaunaySite>) -> Result<Self> {
        ensure(
            raw.len() <= MAX_DELAUNAY_SITES,
            "Delaunay raw-site budget exceeded",
        )?;
        raw.sort_by(|a, b| a.id.cmp(&b.id));
        let mut previous: Option<&str> = None;
        for site in &raw {
            ensure(
                !site.id.is_empty() && site.id.len() <= 1024 && previous != Some(&site.id),
                "Delaunay IDs must be unique, nonempty and at most 1024 bytes",
            )?;
            ensure(
                site.x.is_finite() && site.y.is_finite(),
                "Delaunay coordinates must be finite",
            )?;
            previous = Some(&site.id);
        }
        let mut sites = Vec::new();
        let mut owners = Vec::with_capacity(raw.len());
        let mut positions = BTreeMap::new();
        for site in &raw {
            let key = (zero_bits(site.x), zero_bits(site.y));
            let owner = *positions.entry(key).or_insert_with(|| {
                let index = sites.len();
                sites.push(site.clone());
                index
            });
            owners.push(owner);
        }
        let scale_exponent = normalization(&sites)?;
        let normalized = sites
            .iter()
            .map(|p| [scale2(p.x, scale_exponent), scale2(p.y, scale_exponent)])
            .collect::<Vec<_>>();
        for (site, p) in sites.iter().zip(&normalized) {
            ensure(
                scale2(p[0], -scale_exponent) == site.x && scale2(p[1], -scale_exponent) == site.y,
                "Delaunay scaling would change represented coordinates",
            )?;
            spade::validate_coordinate(p[0])
                .and_then(|()| spade::validate_coordinate(p[1]))
                .map_err(|error| crate::ChartError(format!("Delaunay predicate range: {error}")))?;
        }
        let backend = DelaunayTriangulation::<Point2<f64>>::bulk_load_stable(
            normalized.iter().map(|p| Point2::new(p[0], p[1])).collect(),
        )
        .map_err(|e| crate::ChartError(format!("Delaunay insertion: {e}")))?;
        ensure(
            backend.num_vertices() == sites.len(),
            "Delaunay scaling collapsed distinct sites",
        )?;
        let mut faces = backend
            .inner_faces()
            .map(|face| {
                let mut triangle = face.vertices().map(|v| v.index());
                let minimum = triangle
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, i)| **i)
                    .unwrap()
                    .0;
                triangle.rotate_left(minimum);
                triangle
            })
            .collect::<Vec<_>>();
        faces.sort_unstable();
        let triangles = faces.into_iter().flatten().collect::<Vec<_>>();
        let mut halfedges = vec![None; triangles.len()];
        let mut inedges = vec![None; sites.len()];
        let mut directed = BTreeMap::new();
        for edge in 0..triangles.len() {
            let a = triangles[edge];
            let b = triangles[next(edge)];
            if let Some(&opposite) = directed.get(&(b, a)) {
                halfedges[edge] = Some(opposite);
                halfedges[opposite] = Some(edge);
            }
            directed.insert((a, b), edge);
            inedges[b] = Some(edge);
        }
        let mut links = backend
            .undirected_edges()
            .map(|edge| {
                let mut pair = edge.vertices().map(|v| v.index());
                pair.sort_unstable();
                pair
            })
            .collect::<Vec<_>>();
        links.sort_unstable();
        let mut adjacent = vec![Vec::new(); sites.len()];
        for &[a, b] in &links {
            adjacent[a].push(b);
            adjacent[b].push(a);
        }
        for neighbors in &mut adjacent {
            neighbors.sort_unstable();
        }
        let hull = if triangles.is_empty() {
            // A genuinely collinear chain, with no artificial triangles/jitter.
            let mut chain = (0..sites.len()).collect::<Vec<_>>();
            chain.sort_by(|&a, &b| {
                normalized[a][0]
                    .total_cmp(&normalized[b][0])
                    .then(normalized[a][1].total_cmp(&normalized[b][1]))
                    .then(a.cmp(&b))
            });
            chain
        } else {
            let boundary = halfedges
                .iter()
                .enumerate()
                .filter(|(_, twin)| twin.is_none())
                .map(|(edge, _)| (triangles[edge], (triangles[next(edge)], edge)))
                .collect::<BTreeMap<_, _>>();
            let first = *boundary.keys().next().unwrap();
            let mut hull = Vec::with_capacity(boundary.len());
            let mut current = first;
            loop {
                hull.push(current);
                let &(end, edge) = boundary
                    .get(&current)
                    .ok_or_else(|| crate::ChartError("Delaunay hull is not a cycle".into()))?;
                inedges[end] = Some(edge);
                current = end;
                if current == first {
                    break;
                }
                ensure(hull.len() < boundary.len(), "Delaunay hull budget exceeded")?;
            }
            ensure(
                hull.len() == boundary.len(),
                "Delaunay hull contains disconnected cycles",
            )?;
            hull
        };
        Ok(Self {
            raw,
            sites,
            owners,
            normalized,
            scale_exponent,
            triangles,
            halfedges,
            inedges,
            hull,
            links,
            adjacent,
        })
    }
    pub fn raw_sites(&self) -> &[DelaunaySite] {
        &self.raw
    }
    pub fn sites(&self) -> &[DelaunaySite] {
        &self.sites
    }
    pub fn raw_owners(&self) -> &[usize] {
        &self.owners
    }
    /// Counterclockwise triples, rotated to their smallest index and sorted.
    pub fn triangles(&self) -> &[usize] {
        &self.triangles
    }
    /// Opposite edge index; `None` is a hull edge. Edge e goes from `triangles[e]`
    /// to `triangles[next(e)]`. Twin winding is reversed.
    pub fn halfedges(&self) -> &[Option<usize>] {
        &self.halfedges
    }
    /// Incoming triangle edge; boundary sites prefer their incoming hull edge.
    /// Collinear and singleton sites have no incoming triangle edge.
    pub fn inedges(&self) -> &[Option<usize>] {
        &self.inedges
    }
    /// CCW hull starting at its smallest ID, or geometric order for collinear data.
    pub fn hull(&self) -> &[usize] {
        &self.hull
    }
    /// Unique undirected pairs sorted by their canonical site indices.
    pub fn links(&self) -> &[[usize; 2]] {
        &self.links
    }
    pub fn scale_exponent(&self) -> i32 {
        self.scale_exponent
    }
    pub fn owner_index(&self, raw_id: &str) -> Option<usize> {
        self.raw
            .binary_search_by(|s| s.id.as_str().cmp(raw_id))
            .ok()
            .map(|i| self.owners[i])
    }
    pub fn owner(&self, raw_id: &str) -> Option<&DelaunaySite> {
        self.owner_index(raw_id).map(|i| &self.sites[i])
    }
    /// Duplicates share their owner's neighbors. Missing ID is distinct from an
    /// isolated site's empty neighbor list. Neighbors are sorted by stable ID.
    pub fn neighbors(&self, raw_id: &str) -> Option<Vec<&DelaunaySite>> {
        self.owner_index(raw_id)
            .map(|i| self.adjacent[i].iter().map(|&j| &self.sites[j]).collect())
    }
    /// Fits with a *uniform* scale, preserving the triangulation's Euclidean
    /// metric. Original coordinates remain in sites()/tooltips. Scene's lower
    /// mark/path/label budgets are checked before returning the whole candidate.
    pub fn diagram(
        &self,
        config: &ChartConfig,
        options: &DelaunayDiagramOptions,
    ) -> Result<DelaunayDiagram> {
        let mut scene = config.scene()?;
        ensure(
            options.site_radius.is_finite()
                && (0. ..=256.).contains(&options.site_radius)
                && options.link_width.is_finite()
                && (0. ..=256.).contains(&options.link_width)
                && options.label_size.is_finite()
                && (1. ..=256.).contains(&options.label_size),
            "invalid Delaunay diagram style",
        )?;
        let count = usize::from(options.triangle_fill.is_some()) * self.triangles.len() / 3
            + usize::from(options.link_width > 0.) * self.links.len()
            + usize::from(options.site_radius > 0.) * self.sites.len();
        ensure(
            count <= crate::MAX_MARKS && (!options.labels || self.sites.len() <= crate::MAX_MARKS),
            "Delaunay diagram exceeds scene budget",
        )?;
        let mut min = [0., 0.];
        let mut max = [0., 0.];
        if let Some(first) = self.normalized.first() {
            min = *first;
            max = *first;
        }
        for p in &self.normalized {
            for axis in 0..2 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let span = [max[0] - min[0], max[1] - min[1]];
        let scale = [
            (config.inner_width(), span[0]),
            (config.inner_height(), span[1]),
        ]
        .into_iter()
        .filter(|(_, s)| *s > 0.)
        .map(|(w, s)| w / s)
        .fold(f64::INFINITY, f64::min);
        let scale = if scale.is_finite() { scale } else { 1. };
        let start = [
            config.margin.left + (config.inner_width() - span[0] * scale) / 2.,
            config.margin.top + (config.inner_height() - span[1] * scale) / 2.,
        ];
        let projected = self
            .sites
            .iter()
            .enumerate()
            .map(|(index, site)| DelaunayProjectedSite {
                site_index: index,
                id: site.id.clone(),
                mark_id: format!("delaunay:site:{}:{}", site.id.len(), site.id),
                x: start[0] + (self.normalized[index][0] - min[0]) * scale,
                y: start[1] + (self.normalized[index][1] - min[1]) * scale,
            })
            .collect::<Vec<_>>();
        if let Some(color) = options.triangle_fill {
            for triangle in self.triangles.as_chunks::<3>().0 {
                let p = triangle
                    .iter()
                    .map(|&i| GraphicPoint::new(projected[i].x, projected[i].y))
                    .collect::<Vec<_>>();
                let id = format!(
                    "delaunay:triangle:{}:{}:{}:{}:{}:{}",
                    self.sites[triangle[0]].id.len(),
                    self.sites[triangle[0]].id,
                    self.sites[triangle[1]].id.len(),
                    self.sites[triangle[1]].id,
                    self.sites[triangle[2]].id.len(),
                    self.sites[triangle[2]].id
                );
                let mut mark = Mark::new(
                    id,
                    Geometry::Path(vec![
                        PathCommand::Move(p[0]),
                        PathCommand::Line(p[1]),
                        PathCommand::Line(p[2]),
                        PathCommand::Close,
                    ]),
                    color,
                );
                mark.series = "triangles".into();
                mark.label.clear();
                scene.marks.push(mark);
            }
        }
        if options.link_width > 0. {
            for &[a, b] in &self.links {
                let mut mark = Mark::new(
                    format!(
                        "delaunay:link:{}:{}:{}:{}",
                        self.sites[a].id.len(),
                        self.sites[a].id,
                        self.sites[b].id.len(),
                        self.sites[b].id
                    ),
                    Geometry::Path(vec![
                        PathCommand::Move(GraphicPoint::new(projected[a].x, projected[a].y)),
                        PathCommand::Line(GraphicPoint::new(projected[b].x, projected[b].y)),
                    ]),
                    options.link_color,
                );
                mark.fill = None;
                mark.stroke = Some(GraphicStroke {
                    color: options.link_color,
                    width: options.link_width,
                    cap: StrokeCap::Butt,
                });
                mark.series = "links".into();
                mark.label.clear();
                scene.marks.push(mark);
            }
        }
        for (index, p) in projected.iter().enumerate() {
            let site = &self.sites[index];
            if options.site_radius > 0. {
                let mut mark = Mark::new(
                    &p.mark_id,
                    Geometry::Circle {
                        x: p.x,
                        y: p.y,
                        radius: options.site_radius,
                    },
                    options.site_color,
                );
                mark.series = "sites".into();
                mark.value = self.adjacent[index].len() as f64;
                mark.label = format!(
                    "{}: ({}, {}), {} neighbors",
                    site.id,
                    site.x,
                    site.y,
                    self.adjacent[index].len()
                );
                scene.marks.push(mark);
            }
            if options.labels {
                scene
                    .label_series
                    .insert(scene.labels.len(), "sites".into());
                scene.labels.push(Label {
                    x: p.x + options.site_radius + 4.,
                    y: p.y,
                    text: site.id.clone(),
                    color: config.theme.text,
                    size: options.label_size,
                    anchor: TextAnchor::Start,
                    rotation: 0.,
                });
            }
        }
        if options.legend {
            if options.site_radius > 0. || options.labels {
                scene.legend.push(LegendItem {
                    id: "sites".into(),
                    label: "Sites".into(),
                    color: options.site_color,
                });
            }
            if options.link_width > 0. {
                scene.legend.push(LegendItem {
                    id: "links".into(),
                    label: "Links".into(),
                    color: options.link_color,
                });
            }
            if let Some(color) = options.triangle_fill {
                scene.legend.push(LegendItem {
                    id: "triangles".into(),
                    label: "Triangles".into(),
                    color,
                });
            }
        }
        scene.validate()?;
        Ok(DelaunayDiagram {
            scene,
            sites: projected,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DelaunayDiagramOptions {
    pub site_radius: f64,
    pub site_color: Color,
    pub link_width: f64,
    pub link_color: Color,
    pub triangle_fill: Option<Color>,
    pub labels: bool,
    pub label_size: f64,
    pub legend: bool,
}
impl Default for DelaunayDiagramOptions {
    fn default() -> Self {
        Self {
            site_radius: 5.,
            site_color: Color(40, 110, 180, 255),
            link_width: 1.5,
            link_color: Color(100, 120, 145, 255),
            triangle_fill: None,
            labels: false,
            label_size: 12.,
            legend: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DelaunayProjectedSite {
    pub site_index: usize,
    pub id: String,
    pub mark_id: String,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DelaunayDiagram {
    pub scene: Scene,
    pub sites: Vec<DelaunayProjectedSite>,
}

fn zero_bits(v: f64) -> u64 {
    if v == 0. { 0 } else { v.to_bits() }
}
fn next(edge: usize) -> usize {
    if edge % 3 == 2 { edge - 2 } else { edge + 1 }
}
fn exponent(v: f64) -> i32 {
    let bits = v.abs().to_bits();
    let field = ((bits >> 52) & 2047) as i32;
    if field == 0 {
        63 - bits.leading_zeros() as i32 - 1074
    } else {
        field - 1023
    }
}
fn normalization(sites: &[DelaunaySite]) -> Result<i32> {
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    for p in sites {
        for v in [p.x, p.y] {
            if v != 0. {
                let e = exponent(v);
                lo = lo.min(e);
                hi = hi.max(e);
            }
        }
    }
    if hi == i32::MIN {
        return Ok(0);
    }
    // Floor exponents admit the upper endpoint only provisionally; coordinate
    // validation above rejects its mantissa if it is greater than exactly 2^201.
    let shift = (-hi).max(-142 - lo);
    ensure(
        hi + shift <= 201,
        "Delaunay coordinate exponent span exceeds exact predicate profile",
    )?;
    Ok(shift)
}
fn scale2(mut value: f64, mut exponent: i32) -> f64 {
    while exponent > 512 {
        value *= 2_f64.powi(512);
        exponent -= 512;
    }
    while exponent < -512 {
        value *= 2_f64.powi(-512);
        exponent += 512;
    }
    value * 2_f64.powi(exponent)
}
