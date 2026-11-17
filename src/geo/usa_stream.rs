//! Component-preserving Albers USA open streams. Geographic clipping and
//! adaptive sampling precede the original pixel rectangle postclip.
use std::collections::BTreeMap;

use super::*;
use crate::{
    InteractionAnchor, InteractionBinding, InteractionSite, PickingProfile, SceneInteraction,
};

#[derive(Clone, Debug, PartialEq)]
pub struct GeoUsaLinePart {
    pub region: GeoUsaRegion,
    pub mark_id: String,
    pub geometry: Geometry,
    pub components: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComputedGeoUsaLine {
    pub source_index: usize,
    pub id: String,
    /// The first visible region's real mark; no mark is fabricated for an
    /// invisible source. Remaining region marks alias this physical owner.
    pub mark_id: String,
    /// Effective mark/site series, falling back to id for an empty source.
    pub series: String,
    pub source_series: String,
    pub label: String,
    pub value: Option<f64>,
    pub formatted_value: Option<String>,
    pub parts: Vec<GeoUsaLinePart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoUsaStreamMetadata {
    pub lines: Vec<ComputedGeoUsaLine>,
    /// Adaptive emissions; work also includes charged input/coarse preparation.
    pub sampling: GeoPathStats,
    pub clip_stats: GeoRectClipStats,
    pub source_points: usize,
    pub coarse_points: usize,
    authored_lines: Vec<ComputedGeoUsaLine>,
    authored_marks: Vec<Mark>,
    dimensions: [f64; 2],
    plot: crate::ClipRect,
    interactive: bool,
}

impl GeoUsaStreamMetadata {
    /// Bind this authored layer only. Other Scene marks remain painted but are
    /// not eligible for this binding. PaintedGeometry supports Geometry picking;
    /// nearest-site picking is deliberately unavailable. Each source's anchor
    /// is its first visible path Move (endpoint zero), not its centroid.
    /// Rebinding a changed target layer rejects; accepted host animation uses
    /// the declared path anchor and physical owners in its usual live pipeline.
    pub fn interaction_binding(&self, scene: &Scene) -> Result<InteractionBinding> {
        scene.validate()?;
        ensure(
            [scene.width, scene.height] == self.dimensions && self.lines == self.authored_lines,
            "stale USA line metadata or Scene dimensions",
        )?;
        let lookup = scene
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect::<BTreeMap<_, _>>();
        let group_opacities = scene.mark_group_opacities();
        for expected in &self.authored_marks {
            ensure(
                lookup
                    .get(expected.id.as_str())
                    .is_some_and(|mark| **mark == *expected)
                    && !group_opacities.contains_key(&expected.id),
                "stale USA line authored geometry, paint, clipping or ownership",
            )?;
        }
        let ids = self
            .authored_marks
            .iter()
            .map(|m| m.id.as_str())
            .collect::<BTreeSet<_>>();
        ensure(
            scene
                .paint_marks()
                .filter(|m| ids.contains(m.id.as_str()))
                .map(|m| m.id.as_str())
                .eq(self.authored_marks.iter().map(|m| m.id.as_str())),
            "stale USA line paint order",
        )?;
        let mut binding = InteractionBinding {
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        };
        if self.interactive {
            for line in &self.lines {
                let Some(part) = line.parts.first() else {
                    continue;
                };
                let Geometry::Path(path) = &part.geometry else {
                    unreachable!()
                };
                let Some(Cmd::Move(first)) = path.first() else {
                    return Err(ChartError("USA line anchor is missing".into()));
                };
                let position = [first.x, first.y];
                binding.anchors.insert(
                    line.mark_id.clone(),
                    InteractionAnchor::path_points(&part.geometry, position, &[0])?,
                );
                let mark = lookup[line.mark_id.as_str()];
                binding.sites.push(InteractionSite {
                    mark_id: line.mark_id.clone(),
                    series: line.series.clone(),
                    label: if line.label.is_empty() {
                        line.id.clone()
                    } else {
                        line.label.clone()
                    },
                    value: line.value,
                    formatted_value: line.formatted_value.clone(),
                    color: mark.stroke.unwrap().color,
                    position,
                });
                for part in line.parts.iter().skip(1) {
                    binding
                        .ink_owners
                        .insert(part.mark_id.clone(), line.mark_id.clone());
                }
            }
        }
        binding.validate(scene)?;
        Ok(binding)
    }

    pub fn into_scene_interaction(self, scene: Scene) -> Result<SceneInteraction> {
        let interaction = self.interaction_binding(&scene)?;
        let result = SceneInteraction {
            scene,
            interaction: Some(interaction),
            plot: Some(self.plot),
        };
        result.validate()?;
        Ok(result)
    }
}

struct Budget<'a> {
    options: &'a GeoPathOptions,
    sampling: GeoPathStats,
    clip: GeoRectClipStats,
    source: usize,
    coarse: usize,
}
impl Budget<'_> {
    fn points(&self) -> usize {
        self.source + self.coarse + self.sampling.points + self.clip.output_points
    }
    fn work(&self) -> usize {
        self.sampling.work + self.clip.work
    }
    fn charge(&mut self, work: usize) -> Result<()> {
        self.sampling.work = self
            .sampling
            .work
            .checked_add(work)
            .ok_or_else(|| ChartError("USA stream work overflow".into()))?;
        ensure(
            self.work() <= self.options.max_work,
            "USA stream shared work budget exceeded",
        )
    }
    fn check_points(&self) -> Result<()> {
        ensure(
            self.points() <= self.options.max_points,
            "USA stream shared point budget exceeded",
        )
    }
    fn clip_options(&self) -> Result<GeoRectClipOptions> {
        ensure(
            self.points() < self.options.max_points && self.work() < self.options.max_work,
            "USA stream shared postclip budget exhausted",
        )?;
        Ok(GeoRectClipOptions {
            max_points: self.options.max_points - self.points(),
            max_work: self.options.max_work - self.work(),
        })
    }
    fn absorb_clip(&mut self, s: GeoRectClipStats) -> Result<()> {
        self.clip.input_points += s.input_points;
        self.clip.buffered_points += s.buffered_points;
        self.clip.output_points += s.output_points;
        self.clip.work += s.work;
        self.check_points()?;
        ensure(
            self.work() <= self.options.max_work,
            "USA stream shared work budget exceeded",
        )
    }
}

fn region_name(region: GeoUsaRegion) -> &'static str {
    match region {
        GeoUsaRegion::Lower48 => "lower48",
        GeoUsaRegion::Alaska => "alaska",
        GeoUsaRegion::Hawaii => "hawaii",
    }
}
fn intersect(a: crate::ClipRect, b: crate::ClipRect) -> crate::ClipRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    crate::ClipRect {
        x,
        y,
        width: ((a.x + a.width).min(b.x + b.width) - x).max(0.),
        height: ((a.y + a.height).min(b.y + b.height) - y).max(0.),
    }
}

impl GeoProjectionContext {
    /// Append three continuous geographic streams without refitting the map.
    /// Source, coarse, adaptive and postclip emissions share one point budget;
    /// headers, preparation, adaptive evaluation and clipping share one work
    /// budget. Full original stroke/caps use each part's actual region mask.
    pub fn add_usa_lines(
        &self,
        scene: &mut Scene,
        data: &[GeoLineFeature],
        o: &GeoStreamOptions,
    ) -> Result<GeoUsaStreamMetadata> {
        let usa = self
            .usa_projection()
            .ok_or_else(|| ChartError("USA lines require an Albers USA context".into()))?;
        scene.validate()?;
        ensure(
            [scene.width, scene.height] == self.dimensions,
            "USA line Scene dimensions differ from projection",
        )?;
        ensure(data.len() <= 4096, "USA line feature budget exceeded")?;
        o.style.validate()?;
        o.value_format.validate()?;
        let path = o
            .path
            .as_ref()
            .or(self.path_options.as_ref())
            .cloned()
            .unwrap_or_default();
        path.validate()?;
        let mut budget = Budget {
            options: &path,
            sampling: Default::default(),
            clip: Default::default(),
            source: 0,
            coarse: 0,
        };
        budget.charge(1)?;
        for feature in data {
            budget.charge(1)?;
            ensure(
                feature.lines.len() <= 4096,
                "USA line component budget exceeded",
            )?;
            for line in &feature.lines {
                budget.charge(1)?;
                budget.source = budget
                    .source
                    .checked_add(line.len())
                    .ok_or_else(|| ChartError("USA source point overflow".into()))?;
                ensure(
                    budget.source <= MAX_POINTS,
                    "USA source point budget exceeded",
                )?;
                budget.check_points()?;
            }
        }
        let mut regions = Vec::new();
        for region in GeoUsaRegion::ALL {
            budget.charge(1)?;
            let child = self.usa_component(region)?;
            let remaining = path.max_points.saturating_sub(budget.points());
            let (raw, coarse) = source_metered(data, &child.view, remaining, |n| budget.charge(n))?;
            budget.coarse += coarse;
            budget.check_points()?;
            let seed = GeoPathStats {
                points: budget.points(),
                work: budget.work(),
                deepest: budget.sampling.deepest,
            };
            let (paths, stats, _) =
                projected_seeded(&raw, child.view.projection, child.scale, &path, false, seed)?;
            budget.sampling.points += stats.points - seed.points;
            budget.sampling.work += stats.work - seed.work;
            budget.sampling.deepest = stats.deepest;
            let mut features = Vec::with_capacity(paths.len());
            let dx = child.translate[0] - child.scale * child.center[0];
            let dy = child.translate[1] + child.scale * child.center[1];
            for mut lines in paths {
                budget.charge(1)?;
                for p in lines.iter_mut().flatten() {
                    *p = [dx + child.scale * p[0], dy - child.scale * p[1]];
                }
                let clipped = clip_geo_planar_lines(
                    &lines,
                    usa.region_clip(region),
                    &budget.clip_options()?,
                )?;
                budget.absorb_clip(clipped.stats)?;
                features.push(clipped.lines);
            }
            regions.push((region, features));
        }
        let mut next = scene.clone();
        let old_order = next.draw_items();
        let explicit = !next.draw_order.is_empty();
        let start = next.marks.len();
        let mut commands = next
            .marks
            .iter()
            .map(|m| {
                if let Geometry::Path(p) = &m.geometry {
                    p.len()
                } else {
                    0
                }
            })
            .sum::<usize>();
        let mut metadata = Vec::with_capacity(data.len());
        for (source_index, feature) in data.iter().enumerate() {
            let mark_id = format!("geo-line:{}", feature.id);
            let series = if feature.series.is_empty() {
                &feature.id
            } else {
                &feature.series
            };
            let label = if feature.label.is_empty() {
                &feature.id
            } else {
                &feature.label
            };
            let formatted = feature
                .value
                .map(|v| o.value_format.format(v))
                .transpose()?;
            let style = feature.style.as_ref().unwrap_or(&o.style);
            let mut parts = Vec::new();
            for (region, features) in &regions {
                let lines = &features[source_index];
                if lines.is_empty() {
                    continue;
                }
                let mut path = Vec::new();
                for line in lines {
                    if let Some(p) = line.first() {
                        commands += line.len();
                        ensure(
                            commands <= 200_000,
                            "USA line Scene command budget exceeded",
                        )?;
                        path.push(Cmd::Move(P::new(p[0], p[1])));
                        path.extend(line.iter().skip(1).map(|p| Cmd::Line(P::new(p[0], p[1]))));
                    }
                }
                if path.is_empty() {
                    continue;
                }
                let id = if parts.is_empty() {
                    mark_id.clone()
                } else {
                    format!("{mark_id}:usa:{}", region_name(*region))
                };
                let geometry = Geometry::Path(path);
                let mut mark = Mark::new(&id, geometry.clone(), style.stroke.color);
                mark.fill = None;
                mark.stroke = Some(style.stroke);
                mark.stroke_dash = style.dash.clone();
                mark.opacity = style.opacity;
                mark.series = series.clone();
                mark.value = feature.value.unwrap_or(0.);
                mark.label = if o.interactive {
                    label.clone()
                } else {
                    String::new()
                };
                mark.clip = match (o.clip_plot, o.clip_projection) {
                    (false, false) => None,
                    (true, false) => Some(self.plot),
                    (false, true) => Some(usa.region_clip(*region)),
                    (true, true) => Some(intersect(self.plot, usa.region_clip(*region))),
                };
                parts.push(GeoUsaLinePart {
                    region: *region,
                    mark_id: id,
                    geometry,
                    components: lines.len(),
                });
                ensure(
                    next.marks.len() < crate::MAX_MARKS,
                    "USA line Scene mark budget exceeded",
                )?;
                next.marks.push(mark);
            }
            metadata.push(ComputedGeoUsaLine {
                source_index,
                id: feature.id.clone(),
                mark_id,
                series: series.clone(),
                source_series: feature.series.clone(),
                label: feature.label.clone(),
                value: feature.value,
                formatted_value: formatted,
                parts,
            });
        }
        if o.behind {
            next.draw_order = (start..next.marks.len())
                .map(DrawItem::Mark)
                .chain(old_order)
                .collect();
        } else if explicit {
            next.draw_order
                .extend((start..next.marks.len()).map(DrawItem::Mark));
        }
        next.validate()?;
        let result = GeoUsaStreamMetadata {
            authored_lines: metadata.clone(),
            lines: metadata,
            sampling: budget.sampling,
            clip_stats: budget.clip,
            source_points: budget.source,
            coarse_points: budget.coarse,
            authored_marks: next.marks[start..].to_vec(),
            dimensions: self.dimensions,
            plot: self.plot,
            interactive: o.interactive,
        };
        result.interaction_binding(&next)?;
        *scene = next;
        Ok(result)
    }

    pub fn add_usa_graticule(
        &self,
        scene: &mut Scene,
        grid: &GeoGraticuleOptions,
        o: &GeoStreamOptions,
    ) -> Result<GeoUsaStreamMetadata> {
        ensure(
            self.usa_projection().is_some(),
            "USA graticules require an Albers USA context",
        )?;
        let lines = geo_graticule(grid)?
            .into_iter()
            .map(|g| g.feature)
            .collect::<Vec<_>>();
        let mut options = o.clone();
        options.interactive = false;
        self.add_usa_lines(scene, &lines, &options)
    }
}

impl GeoMapLayout {
    pub fn add_usa_lines(
        &mut self,
        data: &[GeoLineFeature],
        o: &GeoStreamOptions,
    ) -> Result<GeoUsaStreamMetadata> {
        self.projection.add_usa_lines(&mut self.scene, data, o)
    }
    pub fn add_usa_graticule(
        &mut self,
        grid: &GeoGraticuleOptions,
        o: &GeoStreamOptions,
    ) -> Result<GeoUsaStreamMetadata> {
        self.projection.add_usa_graticule(&mut self.scene, grid, o)
    }
}
