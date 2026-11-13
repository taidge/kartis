//! Controlled BoxPlot association, following Nivo 0.99 `useBoxPlot`:
//! the active summary's group OR subgroup selects the related summaries.
//! This is explicit application state, not automatic device-hover registration.
use crate::*;
use std::{collections::BTreeMap, sync::Arc};

/// Original hierarchy-free business identity. An empty subgroup is valid.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxPlotKey {
    pub group: String,
    pub subgroup: String,
}
impl BoxPlotKey {
    pub fn new(group: impl Into<String>, subgroup: impl Into<String>) -> Self {
        Self {
            group: group.into(),
            subgroup: subgroup.into(),
        }
    }
}

/// One compositing factor for the complete summary, including its outliers.
/// Original member opacity (notably BoxPlot's independent fill_opacity) and
/// every RGBA channel remain unchanged. Opacity groups composite once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxPlotInteractionOptions {
    pub opacity: f64,
    pub active_opacity: f64,
    pub inactive_opacity: f64,
}
impl Default for BoxPlotInteractionOptions {
    fn default() -> Self {
        Self {
            opacity: 1.,
            active_opacity: 1.,
            inactive_opacity: 0.25,
        }
    }
}
impl BoxPlotInteractionOptions {
    pub fn validate(&self) -> Result<()> {
        require(
            [self.opacity, self.active_opacity, self.inactive_opacity]
                .iter()
                .all(|x| x.is_finite() && (0. ..=1.).contains(x)),
            "invalid BoxPlot interaction opacity",
        )
    }
}

#[derive(Clone, Debug)]
struct Owner {
    summary: usize,
    outlier: Option<usize>,
}

/// A builder-owned source and an accepted complete public frame.
///
/// Construction always computes from original observations. There is no
/// arbitrary-layout adoption or public signature-refresh operation. Mutating
/// any public layout field makes subsequent interaction/binding/source-update
/// operations reject until that exact accepted frame is restored.
#[derive(Clone)]
pub struct BoxPlotInteraction {
    pub layout: BoxPlotLayout,
    base: Arc<BoxPlotLayout>,
    accepted: Arc<BoxPlotLayout>,
    keys: Vec<BoxPlotKey>,
    by_key: BTreeMap<BoxPlotKey, usize>,
    owners: BTreeMap<String, Owner>,
    members: Vec<Vec<String>>,
    sites: Vec<InteractionSite>,
    anchors: BTreeMap<String, InteractionAnchor>,
    plot: ClipRect,
    options: BoxPlotInteractionOptions,
    active: Option<BoxPlotKey>,
    active_keys: Vec<BoxPlotKey>,
    scale_options: Option<ContinuousScaleOptions>,
    value_scale: Option<ResolvedContinuousScale>,
    // Explicit construction policy also covers legend=false/all-empty builds.
    legend_factory_built: bool,
    color_factory_built: bool,
}
impl std::fmt::Debug for BoxPlotInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = f.debug_struct("BoxPlotInteraction");
        output.field("layout", &self.layout);
        output.field("base", &self.base);
        output.field("accepted", &self.accepted);
        output.field("keys", &self.keys);
        output.field("by_key", &self.by_key);
        output.field("owners", &self.owners);
        output.field("members", &self.members);
        output.field("sites", &self.sites);
        output.field("anchors", &self.anchors);
        output.field("plot", &self.plot);
        output.field("options", &self.options);
        output.field("active", &self.active);
        output.field("active_keys", &self.active_keys);
        output.field("scale_options", &self.scale_options);
        output.field("value_scale", &self.value_scale);
        if self.legend_factory_built {
            output.field("legend_factory_built", &true);
        }
        if self.color_factory_built {
            output.field("color_factory_built", &true);
        }
        output.finish()
    }
}
impl BoxPlotInteraction {
    pub fn new(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        options: &BoxPlotInteractionOptions,
    ) -> Result<Self> {
        options.validate()?;
        Self::from_source(
            c,
            boxplot_layout(c, data, layout)?,
            *options,
            None,
            None,
            false,
            false,
        )
    }

    pub fn with_scale_options(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        value_scale: &ContinuousScaleOptions,
        options: &BoxPlotInteractionOptions,
    ) -> Result<Self> {
        options.validate()?;
        let scaled = boxplot_layout_with_scale_options(c, data, layout, value_scale)?;
        Self::from_source(
            c,
            scaled.layout,
            *options,
            Some(*value_scale),
            Some(scaled.value_scale),
            false,
            false,
        )
    }

    /// Build and cache per-summary legend text. Hover never re-invokes the
    /// borrowed factory. Future source replacements must supply a factory.
    pub fn with_legend_label_factory(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        options: &BoxPlotInteractionOptions,
        factory: &mut BoxPlotLegendLabelFactory<'_>,
    ) -> Result<Self> {
        options.validate()?;
        let source = boxplot_layout_with_legend_label_factory(c, data, layout, factory)?;
        Self::from_source(c, source, *options, None, None, true, false)
    }

    /// Configured continuous value scale with the same cached factory policy.
    pub fn with_scale_options_and_legend_label_factory(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        value_scale: &ContinuousScaleOptions,
        options: &BoxPlotInteractionOptions,
        factory: &mut BoxPlotLegendLabelFactory<'_>,
    ) -> Result<Self> {
        options.validate()?;
        let scaled = boxplot_layout_with_scale_options_and_legend_label_factory(
            c,
            data,
            layout,
            value_scale,
            factory,
        )?;
        Self::from_source(
            c,
            scaled.layout,
            *options,
            Some(*value_scale),
            Some(scaled.value_scale),
            true,
            false,
        )
    }

    /// Build with per-summary source colors and/or cached ByLabel captions.
    /// Callbacks are borrowed for this call, never retained by interaction.
    pub fn with_factories(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        options: &BoxPlotInteractionOptions,
        factories: BoxPlotFactories<'_>,
    ) -> Result<Self> {
        options.validate()?;
        let color_built = factories.color.is_some();
        let legend_built = factories.legend_label.is_some();
        let source = boxplot_layout_with_factories(c, data, layout, factories)?;
        Self::from_source(c, source, *options, None, None, legend_built, color_built)
    }

    /// Per-summary factories using the configured continuous value scale.
    pub fn with_scale_options_and_factories(
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        value_scale: &ContinuousScaleOptions,
        options: &BoxPlotInteractionOptions,
        factories: BoxPlotFactories<'_>,
    ) -> Result<Self> {
        options.validate()?;
        let color_built = factories.color.is_some();
        let legend_built = factories.legend_label.is_some();
        let scaled = boxplot_layout_with_scale_options_and_factories(
            c,
            data,
            layout,
            value_scale,
            factories,
        )?;
        Self::from_source(
            c,
            scaled.layout,
            *options,
            Some(*value_scale),
            Some(scaled.value_scale),
            legend_built,
            color_built,
        )
    }

    fn from_source(
        c: &ChartConfig,
        base: BoxPlotLayout,
        options: BoxPlotInteractionOptions,
        scale_options: Option<ContinuousScaleOptions>,
        value_scale: Option<ResolvedContinuousScale>,
        legend_factory_built: bool,
        color_factory_built: bool,
    ) -> Result<Self> {
        base.scene.validate()?;
        require(
            base.scene.opacity_groups.is_empty(),
            "BoxPlot interaction source already contains opacity groups",
        )?;
        let mark_slots: BTreeMap<_, _> = base
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();
        let mut keys = Vec::with_capacity(base.boxes.len());
        let mut by_key = BTreeMap::new();
        let mut owners = BTreeMap::new();
        let mut sites = Vec::with_capacity(base.boxes.len());
        let mut anchors = BTreeMap::new();
        for (index, summary) in base.boxes.iter().enumerate() {
            let key = BoxPlotKey::new(&summary.group, &summary.subgroup);
            require(
                by_key.insert(key.clone(), index).is_none(),
                "duplicate BoxPlot interaction source key",
            )?;
            keys.push(key);
            let body = &base.scene.marks[*mark_slots
                .get(summary.id.as_str())
                .ok_or_else(|| ChartError("missing BoxPlot source body".into()))?];
            require(
                body.series == boxplot_series_id(&summary.subgroup)
                    && body.value == summary.quantiles[2]
                    && !body.label.is_empty(),
                "BoxPlot source body disagrees with summary",
            )?;
            let color = body
                .fill
                .ok_or_else(|| ChartError("BoxPlot source body has no palette color".into()))?;
            let position = [
                summary.rect.x + summary.rect.width / 2.,
                summary.rect.y + summary.rect.height / 2.,
            ];
            // Existing rounded BoxPlot paths have opposite authored endpoints
            // 0 and 4. This is a coordinate functional, not an added hit area.
            if matches!(body.geometry, Geometry::Path(_)) {
                anchors.insert(
                    body.id.clone(),
                    InteractionAnchor::path_points(&body.geometry, position, &[0, 4])?,
                );
            }
            sites.push(InteractionSite {
                mark_id: body.id.clone(),
                series: body.series.clone(),
                label: body.label.clone(),
                value: Some(summary.quantiles[2]),
                formatted_value: None,
                color,
                position,
            });
            // Enumerate IDs from genuine computed sources. Never parse an
            // arbitrary prefix or interpret a decorative mark's value as data.
            for suffix in ["", ":border", ":median", ":whiskers", ":caps"] {
                let id = format!("{}{suffix}", summary.id);
                if let Some(slot) = mark_slots.get(id.as_str()) {
                    require(
                        base.scene.marks[*slot].series == body.series,
                        "BoxPlot source part has another owner",
                    )?;
                    require(
                        owners
                            .insert(
                                id,
                                Owner {
                                    summary: index,
                                    outlier: None,
                                },
                            )
                            .is_none(),
                        "duplicate BoxPlot source ownership",
                    )?;
                }
            }
            for (outlier, source) in summary.outliers.iter().enumerate() {
                let id = format!(
                    "{}:outlier:{}:{}",
                    summary.id,
                    source.source_id.len(),
                    source.source_id
                );
                if let Some(slot) = mark_slots.get(id.as_str()) {
                    let mark = &base.scene.marks[*slot];
                    require(
                        mark.series == body.series && mark.value == source.value,
                        "BoxPlot source outlier disagrees with summary",
                    )?;
                    require(
                        owners
                            .insert(
                                id,
                                Owner {
                                    summary: index,
                                    outlier: Some(outlier),
                                },
                            )
                            .is_none(),
                        "duplicate BoxPlot outlier ownership",
                    )?;
                }
            }
        }
        let mut members = vec![Vec::new(); base.boxes.len()];
        for item in base.scene.draw_items() {
            if let DrawItem::Mark(index) = item {
                let mark = &base.scene.marks[index];
                if let Some(owner) = owners.get(&mark.id) {
                    members[owner.summary].push(mark.id.clone());
                }
            }
        }
        require(
            members.iter().all(|m| !m.is_empty()),
            "BoxPlot source group is empty",
        )?;
        let source = Arc::new(base);
        let mut output = Self {
            layout: (*source).clone(),
            base: source.clone(),
            accepted: source,
            keys,
            by_key,
            owners,
            members,
            sites,
            anchors,
            plot: ClipRect::plot(c),
            options,
            active: None,
            active_keys: Vec::new(),
            scale_options,
            value_scale,
            legend_factory_built,
            color_factory_built,
        };
        output.set_active(None)?;
        output.interaction_binding()?;
        Ok(output)
    }

    fn validate_accepted(&self) -> Result<()> {
        require(
            self.layout == *self.accepted,
            "BoxPlot accepted layout/source frame was changed",
        )
    }
    pub fn active(&self) -> Option<&BoxPlotKey> {
        self.active.as_ref()
    }
    pub fn active_keys(&self) -> &[BoxPlotKey] {
        &self.active_keys
    }
    pub fn options(&self) -> &BoxPlotInteractionOptions {
        &self.options
    }
    pub fn resolved_value_scale(&self) -> Option<&ResolvedContinuousScale> {
        self.value_scale.as_ref()
    }

    pub fn related(&self, key: &BoxPlotKey) -> Result<Vec<&ComputedBox>> {
        self.validate_accepted()?;
        require(self.by_key.contains_key(key), "unknown active BoxPlot key")?;
        Ok(self
            .layout
            .boxes
            .iter()
            .filter(|b| b.group == key.group || b.subgroup == key.subgroup)
            .collect())
    }
    pub fn box_for_key(&self, key: &BoxPlotKey) -> Result<Option<&ComputedBox>> {
        self.validate_accepted()?;
        Ok(self.by_key.get(key).map(|i| &self.layout.boxes[*i]))
    }
    /// Exact currently emitted source IDs, including outlines, stems, caps and
    /// outliers. Axis/decorative IDs and source outliers with no emitted glyph
    /// have no owner. The returned identity never comes from string parsing.
    pub fn key_for_mark(&self, id: &str) -> Result<Option<&BoxPlotKey>> {
        self.validate_accepted()?;
        Ok(self.owners.get(id).map(|owner| &self.keys[owner.summary]))
    }
    pub fn outlier_for_mark(&self, id: &str) -> Result<Option<&BoxOutlier>> {
        self.validate_accepted()?;
        Ok(self.owners.get(id).and_then(|owner| {
            owner
                .outlier
                .map(|index| &self.layout.boxes[owner.summary].outliers[index])
        }))
    }
    /// Reuse Scene's current geometry/clip picking. Logical navigation sites
    /// below do not invent ink for a constant or zero-extent summary body.
    pub fn hit_test(&self, x: f64, y: f64) -> Result<Option<&BoxPlotKey>> {
        self.validate_accepted()?;
        Ok(self
            .layout
            .scene
            .hit_test(x, y)
            .and_then(|m| self.owners.get(&m.id))
            .map(|o| &self.keys[o.summary]))
    }
    pub fn with_active(&self, active: Option<BoxPlotKey>) -> Result<Self> {
        let mut next = self.clone();
        next.set_active(active)?;
        Ok(next)
    }
    fn prepare_frame(
        &self,
        active: Option<&BoxPlotKey>,
        options: &BoxPlotInteractionOptions,
    ) -> Result<(BoxPlotLayout, Vec<BoxPlotKey>)> {
        options.validate()?;
        if let Some(key) = active {
            require(self.by_key.contains_key(key), "unknown active BoxPlot key")?;
        }
        let mut frame = (*self.base).clone();
        let mut active_keys = Vec::new();
        for (index, key) in self.keys.iter().enumerate() {
            let related =
                active.is_some_and(|a| key.group == a.group || key.subgroup == a.subgroup);
            if related {
                active_keys.push(key.clone());
            }
            let opacity = if active.is_none() {
                options.opacity
            } else if related {
                options.active_opacity
            } else {
                options.inactive_opacity
            };
            if opacity != 1. {
                frame.scene.opacity_groups.push(MarkOpacityGroup {
                    id: frame.boxes[index].id.clone(),
                    members: self.members[index].clone(),
                    opacity,
                });
            }
        }
        frame.scene.validate()?;
        Ok((frame, active_keys))
    }
    /// Transactional controlled association. Unknown/stale source keys reject;
    /// unlike Nivo's arbitrary stale active object, a current box must exist.
    /// No statistics, geometry or color resolver is run during this update.
    pub fn set_active(&mut self, active: Option<BoxPlotKey>) -> Result<()> {
        self.validate_accepted()?;
        let (frame, keys) = self.prepare_frame(active.as_ref(), &self.options)?;
        self.accepted = Arc::new(frame.clone());
        self.layout = frame;
        self.active = active;
        self.active_keys = keys;
        Ok(())
    }
    pub fn set_options(&mut self, options: &BoxPlotInteractionOptions) -> Result<()> {
        self.validate_accepted()?;
        let (frame, keys) = self.prepare_frame(self.active.as_ref(), options)?;
        self.accepted = Arc::new(frame.clone());
        self.layout = frame;
        self.options = *options;
        self.active_keys = keys;
        Ok(())
    }
    /// Wire an observed exact current owned mark ID explicitly. None clears;
    /// unknown/stale or decorative IDs reject without clearing accepted state.
    pub fn set_hovered_mark(&mut self, id: Option<&str>) -> Result<()> {
        self.validate_accepted()?;
        let key = id
            .map(|id| {
                self.owners
                    .get(id)
                    .map(|owner| self.keys[owner.summary].clone())
                    .ok_or_else(|| ChartError("unknown BoxPlot event mark".into()))
            })
            .transpose()?;
        self.set_active(key)
    }
    /// Recompute only on an explicit source update. A still-present active key
    /// is retained; removal/hidden/emptying clears it. Invalid inputs or a
    /// changed current public frame preserve the previous accepted state.
    /// A configured shared continuous scale remains configured here.
    pub fn replace_source(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
    ) -> Result<()> {
        self.validate_accepted()?;
        require(
            !self.color_factory_built,
            "factory-built BoxPlot source replacement requires its factories",
        )?;
        require(
            !self.legend_factory_built,
            "factory-built BoxPlot source replacement requires a legend factory",
        )?;
        let mut next = if let Some(scale) = &self.scale_options {
            Self::with_scale_options(c, data, layout, scale, &self.options)?
        } else {
            Self::new(c, data, layout, &self.options)?
        };
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }
    pub fn replace_source_with_scale_options(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        scale: &ContinuousScaleOptions,
    ) -> Result<()> {
        self.validate_accepted()?;
        require(
            !self.color_factory_built,
            "factory-built BoxPlot source replacement requires its factories",
        )?;
        require(
            !self.legend_factory_built,
            "factory-built BoxPlot source replacement requires a legend factory",
        )?;
        let mut next = Self::with_scale_options(c, data, layout, scale, &self.options)?;
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }

    /// Recompute source and captions with a new borrowed factory, atomically.
    /// The existing configured value scale and still-present active key survive.
    /// Callback side effects before a returned Err are external to this frame.
    pub fn replace_source_with_legend_label_factory(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        factory: &mut BoxPlotLegendLabelFactory<'_>,
    ) -> Result<()> {
        self.validate_accepted()?;
        require(
            !self.color_factory_built,
            "color-factory-built BoxPlot replacement requires a color factory",
        )?;
        let mut next = if let Some(scale) = &self.scale_options {
            Self::with_scale_options_and_legend_label_factory(
                c,
                data,
                layout,
                scale,
                &self.options,
                factory,
            )?
        } else {
            Self::with_legend_label_factory(c, data, layout, &self.options, factory)?
        };
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }

    /// Explicitly replace the configured scale as well as source and factory.
    pub fn replace_source_with_scale_options_and_legend_label_factory(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        scale: &ContinuousScaleOptions,
        factory: &mut BoxPlotLegendLabelFactory<'_>,
    ) -> Result<()> {
        self.validate_accepted()?;
        require(
            !self.color_factory_built,
            "color-factory-built BoxPlot replacement requires a color factory",
        )?;
        let mut next = Self::with_scale_options_and_legend_label_factory(
            c,
            data,
            layout,
            scale,
            &self.options,
            factory,
        )?;
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }

    /// Explicitly rebuild source using fresh borrowed factories. Every factory
    /// kind used to construct this controller must be supplied again, including
    /// an empty/legend=false construction. Hover never re-calls either factory.
    /// The previous accepted frame and active key survive failure; callback
    /// side effects already performed outside the frame are not rolled back.
    pub fn replace_source_with_factories(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        factories: BoxPlotFactories<'_>,
    ) -> Result<()> {
        self.validate_replacement_factories(&factories)?;
        let mut next = if let Some(scale) = &self.scale_options {
            Self::with_scale_options_and_factories(
                c,
                data,
                layout,
                scale,
                &self.options,
                factories,
            )?
        } else {
            Self::with_factories(c, data, layout, &self.options, factories)?
        };
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }

    /// Replace the continuous scale and source with the same explicit policy.
    pub fn replace_source_with_scale_options_and_factories(
        &mut self,
        c: &ChartConfig,
        data: &[BoxObservation],
        layout: &BoxPlotLayoutOptions,
        scale: &ContinuousScaleOptions,
        factories: BoxPlotFactories<'_>,
    ) -> Result<()> {
        self.validate_replacement_factories(&factories)?;
        let mut next = Self::with_scale_options_and_factories(
            c,
            data,
            layout,
            scale,
            &self.options,
            factories,
        )?;
        self.retain_active(&mut next)?;
        *self = next;
        Ok(())
    }

    fn validate_replacement_factories(&self, factories: &BoxPlotFactories<'_>) -> Result<()> {
        self.validate_accepted()?;
        require(
            !self.color_factory_built || factories.color.is_some(),
            "color-factory-built BoxPlot replacement requires a color factory",
        )?;
        require(
            !self.legend_factory_built || factories.legend_label.is_some(),
            "legend-factory-built BoxPlot replacement requires a legend factory",
        )
    }

    fn retain_active(&self, next: &mut Self) -> Result<()> {
        if let Some(key) = self
            .active
            .as_ref()
            .filter(|key| next.by_key.contains_key(*key))
        {
            next.set_active(Some(key.clone()))?;
        }
        Ok(())
    }
    /// Exactly one site per actual computed summary, anchored at its real body
    /// center with median value and the cached resolved source color. Constant bodies keep a
    /// logical site without a fabricated hit rectangle. Parts/outliers resolve
    /// through key_for_mark; outlier_for_mark retains their original raw value.
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        self.validate_accepted()?;
        let binding = InteractionBinding {
            sites: self.sites.clone(),
            anchors: self.anchors.clone(),
            ..Default::default()
        };
        binding.validate(&self.layout.scene)?;
        Ok(binding)
    }
    /// Explicit whole-summary physical ink ownership. The ordinary binding
    /// above remains the original body-site-only legacy profile.
    pub fn whole_parts_interaction_binding(&self) -> Result<InteractionBinding> {
        let mut binding = self.interaction_binding()?;
        binding.picking_profile = PickingProfile::PaintedGeometry;
        binding.ink_owners = self
            .owners
            .iter()
            .filter_map(|(physical, owner)| {
                let body = &self.sites[owner.summary].mark_id;
                if physical == body {
                    None
                } else {
                    Some((physical.clone(), body.clone()))
                }
            })
            .collect();
        binding.part_data = self
            .owners
            .iter()
            .filter_map(|(physical, owner)| {
                owner.outlier.map(|index| {
                    let source = &self.base.boxes[owner.summary].outliers[index];
                    (
                        physical.clone(),
                        InteractionPartDatum {
                            source_id: source.source_id.clone(),
                            value: source.value,
                        },
                    )
                })
            })
            .collect();
        binding.validate(&self.layout.scene)?;
        Ok(binding)
    }
    /// Same accepted Scene and one site per summary, with explicit painted
    /// Geometry picking and the exact current source-owned physical aliases.
    /// Hosts must implement PaintedGeometry, never infer it from alias count.
    pub fn whole_parts_scene_interaction(&self) -> Result<SceneInteraction> {
        let result = SceneInteraction {
            scene: self.layout.scene.clone(),
            interaction: Some(self.whole_parts_interaction_binding()?),
            plot: Some(self.plot),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn scene_interaction(&self) -> Result<SceneInteraction> {
        let result = SceneInteraction {
            scene: self.layout.scene.clone(),
            interaction: Some(self.interaction_binding()?),
            plot: Some(self.plot),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        let binding = self.interaction_binding()?;
        let result = SceneInteraction {
            scene: self.layout.scene,
            interaction: Some(binding),
            plot: Some(self.plot),
        };
        result.validate()?;
        Ok(result)
    }
}
fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
