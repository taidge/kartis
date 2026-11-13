//! Deterministic point mesh, exact nearest-site selection and bounded Voronoi cells.
//! Layout-independent: rebuild explicitly when the accepted scene changes.
use crate::scene::ensure;
use crate::{ClipRect, Geometry, MAX_MARKS, Mark, Result, Scene};

#[derive(Clone, Debug, PartialEq)]
pub struct MeshPoint {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub clip: Option<ClipRect>,
}
#[derive(Clone, Debug)]
struct Node {
    point: usize,
    axis: usize,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}
#[derive(Clone, Debug)]
pub struct PointMesh {
    clip_paths: std::collections::BTreeMap<String, crate::ClipPath>,
    points: Vec<MeshPoint>,
    root: Option<Box<Node>>,
}
fn center(mark: &Mark) -> Option<(f64, f64)> {
    match mark.geometry {
        Geometry::Circle { x, y, .. } => Some((x, y)),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some((x + width / 2., y + height / 2.)),
        _ => None,
    }
}
impl PointMesh {
    /// Build caller-declared logical sites at explicit physical coordinates.
    /// Only the provided IDs are indexed; callers choose eligible/visible data.
    /// Unlike `from_scene`, an anchor is not removed merely because its center
    /// lies outside the mark clip: a rounded endpoint's real glyph can still
    /// extend into that clip. Query points must satisfy the original rectangular
    /// and path clips. This does not infer Path visibility or Exact ink hits.
    pub fn from_scene_at(
        scene: &Scene,
        positions: &std::collections::BTreeMap<String, [f64; 2]>,
    ) -> Result<Self> {
        scene.validate()?;
        ensure(
            positions.len() <= MAX_MARKS,
            "explicit point mesh budget exceeded",
        )?;
        let ids = scene
            .marks
            .iter()
            .map(|mark| mark.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        ensure(
            positions.iter().all(|(id, point)| {
                ids.contains(id.as_str())
                    && point
                        .iter()
                        .all(|value| value.is_finite() && value.abs() <= 1e6)
            }),
            "explicit point mesh refers to missing ID or invalid coordinates",
        )?;
        let mut mesh = Self::new(
            scene
                .marks
                .iter()
                .filter(|mark| {
                    !mark.label.is_empty()
                        && !mark.series.is_empty()
                        && mark.opacity > 0.
                        && (mark.has_fill() || mark.stroke.is_some())
                })
                .filter_map(|mark| {
                    positions.get(&mark.id).map(|point| MeshPoint {
                        id: mark.id.clone(),
                        x: point[0],
                        y: point[1],
                        clip: mark.clip,
                    })
                })
                .collect(),
        )?;
        mesh.clip_paths = scene
            .marks
            .iter()
            .filter(|mark| positions.contains_key(&mark.id))
            .filter_map(|mark| {
                mark.clip_path
                    .as_ref()
                    .map(|clip| (mark.id.clone(), clip.as_ref().clone()))
            })
            .collect();
        Ok(mesh)
    }
    pub fn from_scene(scene: &Scene) -> Result<Self> {
        scene.validate()?;
        let mut mesh = Self::new(
            scene
                .marks
                .iter()
                .filter(|m| {
                    !m.label.is_empty()
                        && !m.series.is_empty()
                        && m.opacity > 0.
                        && (m.has_fill() || m.stroke.is_some())
                })
                .filter_map(|m| {
                    center(m)
                        .filter(|&(x, y)| {
                            m.clip.is_none_or(|c| c.contains(x, y))
                                && m.clip_path.as_ref().is_none_or(|c| c.contains(x, y))
                        })
                        .map(|(x, y)| MeshPoint {
                            id: m.id.clone(),
                            x,
                            y,
                            clip: m.clip,
                        })
                })
                .collect(),
        )?;
        mesh.clip_paths = scene
            .marks
            .iter()
            .filter_map(|m| {
                m.clip_path
                    .as_ref()
                    .map(|c| (m.id.clone(), c.as_ref().clone()))
            })
            .collect();
        Ok(mesh)
    }
    pub fn new(mut points: Vec<MeshPoint>) -> Result<Self> {
        ensure(points.len() <= MAX_MARKS, "point mesh budget exceeded")?;
        points.sort_by(|a, b| a.id.cmp(&b.id));
        for (i, p) in points.iter().enumerate() {
            ensure(
                !p.id.is_empty()
                    && p.id.len() <= 1024
                    && p.x.is_finite()
                    && p.y.is_finite()
                    && p.x.abs() <= 1e6
                    && p.y.abs() <= 1e6
                    && (i == 0 || points[i - 1].id != p.id),
                "invalid mesh point",
            )?;
            if let Some(c) = p.clip {
                c.validate()?;
            }
        }
        fn build(points: &[MeshPoint], mut ids: Vec<usize>, depth: usize) -> Option<Box<Node>> {
            if ids.is_empty() {
                return None;
            }
            let axis = depth % 2;
            ids.sort_by(|a, b| {
                let av = if axis == 0 {
                    points[*a].x
                } else {
                    points[*a].y
                };
                let bv = if axis == 0 {
                    points[*b].x
                } else {
                    points[*b].y
                };
                av.total_cmp(&bv).then_with(|| a.cmp(b))
            });
            let mid = ids.len() / 2;
            Some(Box::new(Node {
                point: ids[mid],
                axis,
                left: build(points, ids[..mid].to_vec(), depth + 1),
                right: build(points, ids[mid + 1..].to_vec(), depth + 1),
            }))
        }
        let root = build(&points, (0..points.len()).collect(), 0);
        Ok(Self {
            points,
            root,
            clip_paths: Default::default(),
        })
    }
    pub fn points(&self) -> &[MeshPoint] {
        &self.points
    }
    /// Euclidean nearest site equals a Voronoi cell query. Ties use lexical ID.
    pub fn nearest(&self, x: f64, y: f64, max_distance: f64) -> Option<&MeshPoint> {
        if !x.is_finite() || !y.is_finite() || max_distance.is_nan() || max_distance < 0. {
            return None;
        }
        let mut best = None;
        let mut distance = max_distance;
        fn visit(
            tree: &Node,
            points: &[MeshPoint],
            clips: &std::collections::BTreeMap<String, crate::ClipPath>,
            x: f64,
            y: f64,
            best: &mut Option<usize>,
            distance: &mut f64,
        ) {
            let p = &points[tree.point];
            let d = (p.x - x).hypot(p.y - y);
            if p.clip.is_none_or(|c| c.contains(x, y))
                && clips.get(&p.id).is_none_or(|c| c.contains(x, y))
                && (d < *distance || (d == *distance && best.is_none_or(|i| tree.point < i)))
            {
                *distance = d;
                *best = Some(tree.point);
            }
            let delta = if tree.axis == 0 { x - p.x } else { y - p.y };
            let (near, far) = if delta < 0. {
                (&tree.left, &tree.right)
            } else {
                (&tree.right, &tree.left)
            };
            if let Some(n) = near {
                visit(n, points, clips, x, y, best, distance);
            }
            if delta.abs() <= *distance
                && let Some(n) = far
            {
                visit(n, points, clips, x, y, best, distance);
            }
        }
        if let Some(root) = &self.root {
            visit(
                root,
                &self.points,
                &self.clip_paths,
                x,
                y,
                &mut best,
                &mut distance,
            );
        }
        best.map(|i| &self.points[i])
    }
    /// Bounded half-plane cells with a common site clip. Different per-site
    /// clips create conditional regions that can be disconnected; this polygon
    /// export rejects them, while `nearest` still supports them exactly.
    /// Limited to 512 sites; interactive queries use the balanced kd-tree.
    pub fn voronoi(&self, bounds: ClipRect) -> Result<Vec<VoronoiCell>> {
        ensure(
            self.clip_paths.is_empty(),
            "Voronoi polygon export does not represent non-rectangular clip paths",
        )?;
        bounds.validate()?;
        ensure(
            self.points.len() <= 512,
            "Voronoi polygon export allows at most 512 sites",
        )?;
        let common_clip = self.points.first().and_then(|p| p.clip);
        ensure(
            self.points.iter().all(|p| p.clip == common_clip),
            "Voronoi polygon export requires identical site clips",
        )?;
        let mut cells = Vec::with_capacity(self.points.len());
        for (i, p) in self.points.iter().enumerate() {
            let mut polygon = vec![
                (bounds.x, bounds.y),
                (bounds.x + bounds.width, bounds.y),
                (bounds.x + bounds.width, bounds.y + bounds.height),
                (bounds.x, bounds.y + bounds.height),
            ];
            if let Some(c) = p.clip {
                polygon = clip_halfplane(polygon, (-1., 0., -c.x));
                polygon = clip_halfplane(polygon, (1., 0., c.x + c.width));
                polygon = clip_halfplane(polygon, (0., -1., -c.y));
                polygon = clip_halfplane(polygon, (0., 1., c.y + c.height));
            }
            for (j, q) in self.points.iter().enumerate() {
                if i == j {
                    continue;
                }
                if p.x == q.x && p.y == q.y {
                    if j < i {
                        polygon.clear();
                        break;
                    }
                    continue;
                }
                let dx = q.x - p.x;
                let dy = q.y - p.y;
                let norm = dx.hypot(dy);
                let a = dx / norm;
                let b = dy / norm;
                let cc = a * (p.x + q.x) / 2. + b * (p.y + q.y) / 2.;
                polygon = clip_halfplane(polygon, (a, b, cc));
                if polygon.is_empty() {
                    break;
                }
            }
            cells.push(VoronoiCell {
                id: p.id.clone(),
                polygon,
            });
        }
        Ok(cells)
    }
}
fn clip_halfplane(input: Vec<(f64, f64)>, (a, b, c): (f64, f64, f64)) -> Vec<(f64, f64)> {
    if input.is_empty() {
        return input;
    }
    let mut output = vec![];
    let mut previous = *input.last().unwrap();
    let mut pv = a * previous.0 + b * previous.1 - c;
    for p in input {
        let v = a * p.0 + b * p.1 - c;
        if (v <= 0.) != (pv <= 0.) {
            let t = pv / (pv - v);
            output.push((
                previous.0 * (1. - t) + p.0 * t,
                previous.1 * (1. - t) + p.1 * t,
            ));
        }
        if v <= 0. {
            output.push(p);
        }
        previous = p;
        pv = v;
    }
    output
}
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiCell {
    pub id: String,
    pub polygon: Vec<(f64, f64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliceAxis {
    X,
    Y,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    pub coordinate: f64,
    pub ids: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct SliceIndex {
    slices: Vec<Slice>,
}
impl SliceIndex {
    pub fn new(mesh: &PointMesh, axis: SliceAxis) -> Self {
        let mut points: Vec<_> = mesh.points.iter().collect();
        let coordinate = |p: &&MeshPoint| if axis == SliceAxis::X { p.x } else { p.y };
        points.sort_by(|a, b| {
            coordinate(a)
                .total_cmp(&coordinate(b))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut slices: Vec<Slice> = vec![];
        for p in points {
            let value = if axis == SliceAxis::X { p.x } else { p.y };
            if let Some(last) = slices.last_mut().filter(|s| s.coordinate == value) {
                last.ids.push(p.id.clone());
            } else {
                slices.push(Slice {
                    coordinate: value,
                    ids: vec![p.id.clone()],
                });
            }
        }
        Self { slices }
    }
    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }
    pub fn nearest(&self, coordinate: f64, max_distance: f64) -> Option<&Slice> {
        if !coordinate.is_finite() || max_distance.is_nan() || max_distance < 0. {
            return None;
        }
        let i = self.slices.partition_point(|s| s.coordinate < coordinate);
        [i.checked_sub(1), Some(i)]
            .into_iter()
            .flatten()
            .filter_map(|i| self.slices.get(i))
            .filter(|s| (s.coordinate - coordinate).abs() <= max_distance)
            .min_by(|a, b| {
                (a.coordinate - coordinate)
                    .abs()
                    .total_cmp(&(b.coordinate - coordinate).abs())
                    .then_with(|| a.coordinate.total_cmp(&b.coordinate))
            })
    }
}
