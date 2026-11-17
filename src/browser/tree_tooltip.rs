//! Tree-owned tooltip selection; it never changes a Surface hit or datum.
use super::*;
use crate::browser::tree_tooltip_host::{
    Component, Content, LinkComponent, LinkContent, PanelRequest, Request as PresentationRequest,
    Stage,
};

#[derive(Clone, PartialEq)]
struct Selection {
    id: String,
    client: [f64; 2],
}

#[derive(Clone, PartialEq)]
struct LinkSelection {
    id: String,
    client: [f64; 2],
    origin: BrowserTreeMouseOrigin,
    occurrence: Option<BrowserTreeMouseOccurrence>,
    binding: Option<u64>,
}
#[derive(Default)]
pub(super) struct TooltipController {
    desired: TooltipConfig,
    accepted: TooltipConfig,
    pending_config: bool,
    selection: Option<Selection>,
    cached: Option<(u64, String, TreeTooltipData)>,
    link_selection: Option<LinkSelection>,
    link_cached: Option<(u64, String, TreeLinkTooltipData)>,
}

#[derive(Clone, Default)]
pub(super) struct TooltipConfig {
    options: Option<BrowserTreeTooltipOptions>,
    component: Option<Rc<Component>>,
    link_options: Option<BrowserTreeLinkTooltipOptions>,
    link_component: Option<Rc<LinkComponent>>,
}

impl TooltipController {
    fn configure(&mut self) {
        if self.accepted.options.is_none() {
            self.selection = None;
            self.cached = None;
        }
        self.pending_config = true;
    }
}

impl BrowserTreeHandle {
    /// Enable the Tree's native-node tooltip presentation. Existing mount
    /// defaults remain unchanged until this API or the component API is used.
    pub fn set_node_tooltip_options(&self, options: BrowserTreeTooltipOptions) -> Result<()> {
        options.validate()?;
        let driver = self.driver()?;
        driver.request()?;
        {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip.desired.options = Some(options);
            tooltip.configure();
        }
        driver.tooltip_dirty.set(true);
        Ok(())
    }
    /// A genuine retained Glory subtree. Its optional frame changes at the
    /// accepted source boundary; changing hovered nodes keeps local state.
    /// Use a new revision to rebuild a previously accepted component.
    pub fn set_node_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserTreeTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip.desired.options.get_or_insert_with(Default::default);
            let retired = tooltip
                .desired
                .component
                .replace(Component::new(revision, factory));
            tooltip.configure();
            retired
        };
        drop(retired);
        driver.tooltip_dirty.set(true);
        Ok(())
    }
    /// Return to safe Tree text while retaining the native target/options.
    pub fn clear_node_tooltip_component(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip.desired.options.get_or_insert_with(Default::default);
            let retired = tooltip.desired.component.take();
            tooltip.configure();
            retired
        };
        drop(retired);
        driver.tooltip_dirty.set(true);
        Ok(())
    }
    /// Restore the existing Core tooltip profile and clear Tree presentation
    /// selection. This does not clear or select an activity/current node.
    pub fn reset_node_tooltip(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip.desired.options = None;
            let retired = tooltip.desired.component.take();
            tooltip.configure();
            retired
        };
        drop(retired);
        driver.tooltip_dirty.set(true);
        Ok(())
    }
    /// Read only the accepted Tree presentation. Constructors and native paint
    /// callbacks can inspect this without mutating or flushing the owner.
    pub fn node_tooltip(&self) -> Result<Option<TreeTooltipFrame>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        if !driver.alive.get() {
            return Ok(None);
        }
        Ok(driver.core()?.tree_tooltip_frame())
    }
}

impl BrowserTree {
    pub fn node_tooltip(&self) -> Result<Option<TreeTooltipFrame>> {
        self.handle().node_tooltip()
    }
}

impl Driver {
    pub(super) fn select_tooltip(&self, intent: PointerIntent, id: Option<&str>) {
        let mut tooltip = self.tooltip.borrow_mut();
        let config = if tooltip.pending_config {
            &tooltip.desired
        } else {
            &tooltip.accepted
        };
        if config.options.is_none() || tooltip.link_selection.is_some() {
            return;
        }
        let next = match intent {
            PointerIntent::Move(client, true)
            | PointerIntent::SvgMove(client, true, _)
            | PointerIntent::MouseNode {
                client,
                trusted: true,
                ..
            } => id.map(|id| Selection {
                id: id.to_owned(),
                client,
            }),
            PointerIntent::Move(_, false)
            | PointerIntent::SvgMove(_, false, _)
            | PointerIntent::MouseNode { trusted: false, .. } => return,
            PointerIntent::Leave => None,
        };
        if tooltip.selection != next {
            let same_id = tooltip
                .selection
                .as_ref()
                .zip(next.as_ref())
                .is_some_and(|(a, b)| a.id == b.id);
            tooltip.selection = next;
            if !same_id {
                tooltip.cached = None;
            }
            self.tooltip_dirty.set(true);
        }
    }

    /// Consume a configuration attempt before fallible source/Widget work.
    /// A rejected desired factory is retried only by an explicit request;
    /// subsequent native intent continues to use the accepted presentation.
    pub(super) fn take_tooltip_config(&self, explicit: bool) -> TooltipConfig {
        let mut tooltip = self.tooltip.borrow_mut();
        let desired = std::mem::take(&mut tooltip.pending_config) || explicit;
        if desired {
            tooltip.desired.clone()
        } else {
            tooltip.accepted.clone()
        }
    }

    pub(super) fn accept_tooltip_config(&self, config: TooltipConfig) {
        let retired = {
            let mut tooltip = self.tooltip.borrow_mut();
            if config.options.is_none() {
                tooltip.selection = None;
                tooltip.cached = None;
            }
            if config.link_options.is_none() {
                tooltip.link_selection = None;
                tooltip.link_cached = None;
            }
            std::mem::replace(&mut tooltip.accepted, config)
        };
        drop(retired);
        self.prune_mouse_interest();
    }

    /// Synchronize only exclusions against accepted state. Never redirect to
    /// another source index merely because the source order/visibility changed.
    pub(super) fn exclude_tooltip(&self) -> Result<()> {
        let model = self.model()?;
        let accepted = self.accepted.borrow();
        let mut tooltip = self.tooltip.borrow_mut();
        if tooltip.selection.as_ref().is_some_and(|selection| {
            model.hidden().contains(&selection.id)
                || !accepted
                    .layout
                    .layout
                    .source()
                    .iter()
                    .any(|d| d.id == selection.id)
        }) {
            tooltip.selection = None;
            tooltip.cached = None;
            self.tooltip_dirty.set(true);
        }
        let invalid_link = tooltip.link_selection.as_ref().is_some_and(|selection| {
            let link =
                accepted.layout.layout.links.iter().find(|link| {
                    accepted.layout.layout.source()[link.target_index].id == selection.id
                });
            if let Some(binding) = selection.binding {
                return link.is_none() || !self.mouse_link_binding_valid(binding, &selection.id);
            }
            let mark = link
                .and_then(|link| link.mark_id.as_deref())
                .and_then(|id| model.scene().marks.iter().find(|mark| mark.id == id));
            let ink = mark.is_some_and(|mark| {
                mark.opacity > 0.
                    && !model.hidden().contains(&mark.series)
                    && (mark.fill.is_some_and(|Color(_, _, _, a)| a > 0)
                        || mark
                            .stroke
                            .is_some_and(|stroke| stroke.width > 0. && stroke.color.3 > 0))
            });
            let occurrence_valid = selection.occurrence.as_ref().is_none_or(|old| {
                self.core
                    .upgrade()
                    .and_then(|core| core.tree_svg_stage())
                    .is_some_and(|stage| {
                        stage.plan().layers().iter().any(|occurrence| {
                            occurrence.layer.key == old.key
                                && occurrence.layer.kind == TreeSvgLayerKind::Links
                        })
                    })
            });
            !ink || !occurrence_valid
        });
        if invalid_link {
            tooltip.link_selection = None;
            tooltip.link_cached = None;
            self.link_tooltip_dirty.set(true);
        }
        Ok(())
    }

    pub(super) fn prepare_tooltip(
        &self,
        candidate: &Accepted,
        source_update: bool,
        config: &TooltipConfig,
        suppress_selection: bool,
        svg_stage: Option<&super::super::tree_svg_frame::Stage>,
    ) -> Result<Rc<Stage>> {
        let core = self.core()?;
        let (model, enabled) = {
            let state = core.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree chart is stopped".into()))?;
            let mut model = state.model.clone();
            if source_update {
                model.set_layout(tree_activity_frame(&candidate.layout)?)?;
            }
            (model, state.options.tooltip)
        };
        let (options, component, frame) = {
            let mut tooltip = self.tooltip.borrow_mut();
            let options = config.options;
            let component = config.component.clone();
            let selection = tooltip.selection.clone().filter(|s| {
                !suppress_selection
                    && !model.hidden().contains(&s.id)
                    && candidate
                        .layout
                        .layout
                        .source()
                        .iter()
                        .any(|d| d.id == s.id)
            });
            let frame = if let Some(options) = options.filter(|_| enabled) {
                if let Some(selection) = selection {
                    let cache_valid = tooltip.cached.as_ref().is_some_and(|(generation, id, _)| {
                        *generation == candidate.generation && id == &selection.id
                    });
                    if !cache_valid {
                        tooltip.cached = Some((
                            candidate.generation,
                            selection.id.clone(),
                            TreeTooltipData::new(
                                candidate.request.config.clone(),
                                candidate.layout.clone(),
                                &selection.id,
                            )?,
                        ));
                    }
                    let data = tooltip
                        .cached
                        .as_ref()
                        .expect("captured tooltip data")
                        .2
                        .clone();
                    let point = model
                        .paint_transform()
                        .project((data.anchor()[0], data.anchor()[1]))?;
                    Some(TreeTooltipFrame {
                        data,
                        source_generation: candidate.generation,
                        pointer_client: Some(selection.client),
                        viewport_anchor: [point.0, point.1],
                        anchor: options.anchor,
                    })
                } else {
                    None
                }
            } else {
                None
            };
            (options, component, frame)
        };
        let content = match (options, component) {
            (None, _) => Content::Unmanaged,
            (Some(_), Some(component)) => Content::Component(component),
            (Some(_), None) => {
                let options = {
                    let state = core.state.borrow();
                    state
                        .as_ref()
                        .ok_or_else(|| ChartError("Tree chart is stopped".into()))?
                        .options
                        .tooltip_options
                        .clone()
                };
                Content::Text(
                    frame
                        .as_ref()
                        .map(|f| f.data.text(&options))
                        .transpose()?
                        .unwrap_or_default(),
                )
            }
        };
        let link = self.prepare_link_tooltip(
            candidate,
            config,
            &model,
            enabled,
            suppress_selection,
            svg_stage,
        )?;
        let previous = self.building.replace(true);
        let stage = core.prepare_tree_tooltips(PresentationRequest {
            source_generation: candidate.generation,
            node: PanelRequest {
                frame,
                options: options.unwrap_or_default(),
                content,
            },
            link,
        });
        self.building.set(previous);
        scene::ensure(
            self.alive.get() && core.alive.get(),
            "Tree stopped during tooltip construction",
        )?;
        stage
    }

    pub(super) fn clear_tooltip_controller(&self) {
        let retired = self.tooltip.replace(TooltipController::default());
        drop(retired);
        self.tooltip_dirty.set(false);
        self.link_tooltip_dirty.set(false);
    }

    pub(super) fn retire_tooltip_selection(&self) {
        let mut tooltip = self.tooltip.borrow_mut();
        tooltip.selection = None;
        tooltip.cached = None;
        tooltip.link_selection = None;
        tooltip.link_cached = None;
    }
}

impl BrowserTreeHandle {
    /// Enable cursor-positioned SVG link presentation. This does not select a node.
    pub fn set_link_tooltip_options(&self, options: BrowserTreeLinkTooltipOptions) -> Result<()> {
        options.validate()?;
        let driver = self.driver()?;
        driver.admit_link_tooltip()?;
        driver.listen_mouse()?;
        driver.request()?;
        let mut tooltip = driver.tooltip.borrow_mut();
        tooltip.desired.link_options = Some(options);
        tooltip.configure_link();
        driver.link_tooltip_dirty.set(true);
        Ok(())
    }
    /// Retain a genuine Glory link tooltip across links and node/link switches.
    /// A new revision rebuilds this role only; the same revision keeps its scope.
    pub fn set_link_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserTreeLinkTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        let driver = self.driver()?;
        driver.admit_link_tooltip()?;
        driver.listen_mouse()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip
                .desired
                .link_options
                .get_or_insert_with(Default::default);
            let retired = tooltip
                .desired
                .link_component
                .replace(LinkComponent::new(revision, factory));
            tooltip.configure_link();
            retired
        };
        drop(retired);
        driver.link_tooltip_dirty.set(true);
        Ok(())
    }
    /// Select safe endpoint text for the configured SVG link target.
    pub fn clear_link_tooltip_component(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.admit_link_tooltip()?;
        driver.listen_mouse()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip
                .desired
                .link_options
                .get_or_insert_with(Default::default);
            let retired = tooltip.desired.link_component.take();
            tooltip.configure_link();
            retired
        };
        drop(retired);
        driver.link_tooltip_dirty.set(true);
        Ok(())
    }
    /// Disable link management without changing the independent node policy.
    pub fn reset_link_tooltip(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.admit_link_tooltip()?;
        driver.request()?;
        let retired = {
            let mut tooltip = driver.tooltip.borrow_mut();
            tooltip.desired.link_options = None;
            let retired = tooltip.desired.link_component.take();
            tooltip.pending_config = true;
            retired
        };
        drop(retired);
        driver.link_tooltip_dirty.set(true);
        Ok(())
    }
    pub fn link_tooltip(&self) -> Result<Option<TreeLinkTooltipFrame>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        if !driver.alive.get() {
            return Ok(None);
        }
        Ok(driver.core()?.tree_link_tooltip_frame())
    }
}
impl BrowserTree {
    pub fn set_link_tooltip_options(&self, options: BrowserTreeLinkTooltipOptions) -> Result<()> {
        self.handle().set_link_tooltip_options(options)
    }
    pub fn set_link_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserTreeLinkTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        self.handle().set_link_tooltip_component(revision, factory)
    }
    pub fn clear_link_tooltip_component(&self) -> Result<()> {
        self.handle().clear_link_tooltip_component()
    }
    pub fn reset_link_tooltip(&self) -> Result<()> {
        self.handle().reset_link_tooltip()
    }
    pub fn link_tooltip(&self) -> Result<Option<TreeLinkTooltipFrame>> {
        self.handle().link_tooltip()
    }
}
impl TooltipController {
    fn configure_link(&mut self) {
        if self.accepted.link_options.is_none() {
            self.link_selection = None;
            self.link_cached = None;
        }
        self.pending_config = true;
    }
}
impl Driver {
    fn admit_link_tooltip(&self) -> Result<()> {
        scene::ensure(
            self.chart()?.snapshot()?.renderer == BrowserRenderer::Svg,
            "Tree link tooltip requires SVG renderer",
        )
    }
    pub(super) fn link_tooltip_interested(&self) -> bool {
        let tooltip = self.tooltip.borrow();
        (if tooltip.pending_config {
            &tooltip.desired
        } else {
            &tooltip.accepted
        })
        .link_options
        .is_some()
    }
    /// Reserve actual link presentation independently from activity/current.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn select_link_tooltip(
        &self,
        kind: BrowserTreeMouseKind,
        id: Option<&str>,
        client: [f64; 2],
        origin: BrowserTreeMouseOrigin,
        occurrence: Option<BrowserTreeMouseOccurrence>,
        binding: Option<u64>,
        trusted: bool,
    ) {
        if !trusted || !self.link_tooltip_interested() {
            return;
        }
        let mut tooltip = self.tooltip.borrow_mut();
        let next = match kind {
            BrowserTreeMouseKind::Enter | BrowserTreeMouseKind::Move => {
                id.map(|id| LinkSelection {
                    id: id.into(),
                    client,
                    origin,
                    occurrence: occurrence.clone(),
                    binding,
                })
            }
            BrowserTreeMouseKind::Leave => {
                let public = self
                    .core
                    .upgrade()
                    .and_then(|core| core.tree_link_tooltip_frame());
                let matches = tooltip.link_selection.as_ref().is_some_and(|old| {
                    id == Some(old.id.as_str())
                        && old.binding == binding
                        && public
                            .as_ref()
                            .and_then(|frame| frame.occurrence.as_ref())
                            .or(old.occurrence.as_ref())
                            .map(|o| (&o.key, o.publication))
                            == occurrence.as_ref().map(|o| (&o.key, o.publication))
                });
                if matches {
                    None
                } else {
                    return;
                }
            }
            _ => return,
        };
        if tooltip.link_selection != next {
            let same = tooltip
                .link_selection
                .as_ref()
                .zip(next.as_ref())
                .is_some_and(|(a, b)| a.id == b.id);
            tooltip.link_selection = next;
            if !same {
                tooltip.link_cached = None;
            }
            if tooltip.link_selection.is_some() {
                tooltip.selection = None;
                tooltip.cached = None;
            }
            self.link_tooltip_dirty.set(true);
        }
    }
    fn prepare_link_tooltip(
        &self,
        candidate: &Accepted,
        config: &TooltipConfig,
        model: &SurfaceModel,
        enabled: bool,
        suppress_selection: bool,
        svg_stage: Option<&super::super::tree_svg_frame::Stage>,
    ) -> Result<PanelRequest<TreeLinkTooltipFrame>> {
        let mut tooltip = self.tooltip.borrow_mut();
        let selection = tooltip.link_selection.clone().filter(|selection| {
            !suppress_selection
                && selection
                    .binding
                    .is_none_or(|id| self.mouse_link_binding_valid(id, &selection.id))
        });
        let mut frame = None;
        let current_svg = self.core()?.tree_svg_stage();
        let presentation = svg_stage.or(current_svg.as_deref());
        if enabled
            && let Some(options) = config.link_options
            && let Some(selection) = selection
        {
            let index = candidate.layout.layout.links.iter().position(|link| {
                candidate.layout.layout.source()[link.target_index].id == selection.id
            });
            if index.is_some() {
                let valid = tooltip
                    .link_cached
                    .as_ref()
                    .is_some_and(|(generation, id, _)| {
                        *generation == candidate.generation && id == &selection.id
                    });
                if !valid {
                    tooltip.link_cached = Some((
                        candidate.generation,
                        selection.id.clone(),
                        TreeLinkTooltipData::new(
                            candidate.request.config.clone(),
                            candidate.layout.clone(),
                            &selection.id,
                        )?,
                    ));
                }
                let data = tooltip
                    .link_cached
                    .as_ref()
                    .expect("captured link tooltip")
                    .2
                    .clone();
                let occurrence = match (&selection.occurrence, presentation) {
                    (Some(old), Some(stage)) => stage.plan().layers().iter().find(|occurrence|
                        occurrence.layer.key == old.key && (selection.binding.is_some()
                            || (occurrence.layer.kind == TreeSvgLayerKind::Links && data.mark().is_some_and(|mark|
                                occurrence.items().iter().any(|item| matches!(item, DrawItem::Mark(index) if stage.plan().frame().layout().scene.marks[*index].id == mark.id))))))
                        .map(|occurrence| BrowserTreeMouseOccurrence { key: old.key.clone(), index: occurrence.index, publication: stage.publication() }),
                    (None, None) => None,
                    _ => None,
                };
                let occurrence_valid = selection.occurrence.is_none() || occurrence.is_some();
                let actual_mark = data.mark().and_then(|mark| {
                    presentation.map_or(Some(mark), |stage| stage.tooltip_mark(mark))
                });
                let ink = selection.binding.is_some()
                    || actual_mark.is_some_and(|mark| {
                        mark.opacity > 0.
                            && !model.hidden().contains(&mark.series)
                            && (mark.fill.is_some_and(|Color(_, _, _, a)| a > 0)
                                || mark
                                    .stroke
                                    .is_some_and(|stroke| stroke.width > 0. && stroke.color.3 > 0))
                    });
                if ink && occurrence_valid {
                    let bounds = self.surface.get_bounding_client_rect();
                    let viewport_anchor = if bounds.width() > 0. && bounds.height() > 0. {
                        [
                            (selection.client[0] - bounds.x()) / bounds.width()
                                * model.scene().width,
                            (selection.client[1] - bounds.y()) / bounds.height()
                                * model.scene().height,
                        ]
                    } else {
                        [0., 0.]
                    };
                    frame = Some(TreeLinkTooltipFrame {
                        data,
                        source_generation: candidate.generation,
                        pointer_client: selection.client,
                        viewport_anchor,
                        requested_anchor: options.anchor,
                        anchor: options.anchor,
                        occurrence,
                        origin: selection.origin,
                    });
                }
            }
        }
        let content = if !enabled || config.link_options.is_none() {
            LinkContent::Unmanaged
        } else if let Some(component) = &config.link_component {
            LinkContent::Component(component.clone())
        } else {
            let core = self.core()?;
            let state = core.state.borrow();
            let appearance = &state
                .as_ref()
                .ok_or_else(|| ChartError("Tree chart stopped".into()))?
                .options
                .tooltip_options;
            LinkContent::Text(
                frame
                    .as_ref()
                    .map(|f| f.data.text(appearance))
                    .transpose()?
                    .unwrap_or_default(),
            )
        };
        Ok(PanelRequest {
            frame,
            options: config.link_options.unwrap_or_default().panel(),
            content,
        })
    }
}
