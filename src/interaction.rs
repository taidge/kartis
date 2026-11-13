//! Renderer-independent selection, legends and pointer coordinate conversion.
use crate::{ChartError, Result, Scene};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Interaction {
    pub hovered: Option<String>,
    pub selected: Option<String>,
    pub hidden: BTreeSet<String>,
}
impl Interaction {
    pub fn pointer(&mut self, scene: &Scene, x: f64, y: f64) -> Option<&str> {
        let groups = scene.mark_group_opacities();
        self.hovered = if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || y < 0.
            || x > scene.width
            || y > scene.height
        {
            None
        } else {
            scene
                .paint_marks()
                .rev()
                .filter(|m| {
                    !m.label.is_empty()
                        && !self.hidden.contains(&m.series)
                        && m.opacity > 0.
                        && groups.get(&m.id).is_none_or(|o| *o > 0.)
                })
                .find(|m| crate::scene::contains(m, x, y))
                .map(|m| m.id.clone())
        };
        self.hovered.as_deref()
    }
    pub fn activate(&mut self) {
        self.selected = self.hovered.clone();
    }
    pub fn toggle_series(&mut self, id: &str) {
        if !self.hidden.remove(id) {
            self.hidden.insert(id.into());
        }
    }
    /// Keyboard traversal follows the deterministic drawing order.
    pub fn step(&mut self, scene: &Scene, forward: bool) -> Option<&str> {
        let groups = scene.mark_group_opacities();
        let ids: Vec<_> = scene
            .paint_marks()
            .filter(|m| {
                !m.label.is_empty()
                    && m.opacity > 0.
                    && groups.get(&m.id).is_none_or(|o| *o > 0.)
                    && !self.hidden.contains(&m.series)
            })
            .map(|m| m.id.as_str())
            .collect();
        if ids.is_empty() {
            self.selected = None;
            return None;
        }
        let index = self
            .selected
            .as_deref()
            .and_then(|id| ids.iter().position(|v| *v == id));
        let next = match index {
            None => {
                if forward {
                    0
                } else {
                    ids.len() - 1
                }
            }
            Some(i) => {
                if forward {
                    (i + 1) % ids.len()
                } else {
                    (i + ids.len() - 1) % ids.len()
                }
            }
        };
        self.selected = Some(ids[next].into());
        self.selected.as_deref()
    }
    pub fn reconcile(&mut self, scene: &Scene) {
        let groups = scene.mark_group_opacities();
        for selected in [&mut self.hovered, &mut self.selected] {
            if selected.as_ref().is_some_and(|id| {
                !scene.marks.iter().any(|m| {
                    &m.id == id
                        && !m.label.is_empty()
                        && m.opacity > 0.
                        && groups.get(&m.id).is_none_or(|o| *o > 0.)
                        && !self.hidden.contains(&m.series)
                })
            }) {
                *selected = None;
            }
        }
    }
}
/// Map CSS pointer coordinates into the scene, independent of devicePixelRatio.
pub fn pointer_coordinates(
    scene: &Scene,
    client: (f64, f64),
    bounds: (f64, f64, f64, f64),
) -> Result<(f64, f64)> {
    let (left, top, width, height) = bounds;
    if ![left, top, width, height, client.0, client.1]
        .iter()
        .all(|v| v.is_finite())
        || width <= 0.
        || height <= 0.
    {
        return Err(ChartError("invalid pointer bounds".into()));
    }
    let point = (
        (client.0 - left) * scene.width / width,
        (client.1 - top) * scene.height / height,
    );
    if !point.0.is_finite()
        || !point.1.is_finite()
        || !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.
        || scene.height <= 0.
    {
        return Err(ChartError(
            "pointer coordinate overflow or invalid scene dimensions".into(),
        ));
    }
    Ok(point)
}
