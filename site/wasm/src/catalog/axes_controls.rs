//! Axis controls for the showcase layouts. Axis decorations have stable mark IDs.
//! Tick labels are distinguished from datum labels by ownership and chart layout.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint, PathCommand};
use kartis::{DrawItem, Geometry, Label, Mark, Result, Scene, TextAnchor};

use super::{Control, Settings};

const VISIBLE: &[(&str, &str)] = &[("show", "Show"), ("hide", "Hide")];
const OPTIONAL: &[(&str, &str)] = &[("hide", "Hide"), ("show", "Show")];
const X: Control = Control {
    key: "axis-x",
    label: "X axis",
    choices: VISIBLE,
};
const Y: Control = Control {
    key: "axis-y",
    label: "Y axis",
    choices: VISIBLE,
};
const GRID: Control = Control {
    key: "grid-visible",
    label: "Grid lines",
    choices: VISIBLE,
};
const X_GRID: Control = Control {
    key: "grid-x",
    label: "X grid lines",
    choices: OPTIONAL,
};
const Y_GRID: Control = Control {
    key: "grid-visible",
    label: "Y grid lines",
    choices: VISIBLE,
};
const SINGLE: Control = Control {
    key: "axis-visible",
    label: "Axes",
    choices: VISIBLE,
};
const CATEGORIES: Control = Control {
    key: "axis-visible",
    label: "Category axis labels",
    choices: VISIBLE,
};

pub(super) fn controls(kind: &str) -> &'static [Control] {
    match kind {
        "line" | "area" | "scatter" | "stream" => &[X, Y, X_GRID, Y_GRID],
        "bump" | "bar" | "boxplot" => &[X, Y, GRID],
        "area-bump" => &[X, GRID],
        "heatmap" | "swarm" => &[X, Y],
        "bullet" | "parallel-coordinates" => &[SINGLE],
        "marimekko" => &[CATEGORIES],
        "radar" | "polar-bar" => &[CATEGORIES, GRID],
        "geo-map" => &[Control {
            key: "grid-visible",
            label: "Graticule",
            choices: VISIBLE,
        }],
        _ => &[],
    }
}

pub(super) fn retain_labels(scene: &mut Scene, keep: impl Fn(usize, &Label) -> bool) {
    let mut slots = BTreeMap::new();
    scene.labels = std::mem::take(&mut scene.labels)
        .into_iter()
        .enumerate()
        .filter_map(|(i, label)| {
            if !keep(i, &label) {
                return None;
            }
            slots.insert(i, slots.len());
            Some(label)
        })
        .collect();
    scene.draw_order.retain_mut(|item| match item {
        DrawItem::Label(i) => slots.get(i).is_some_and(|new| {
            *i = *new;
            true
        }),
        DrawItem::Mark(_) => true,
    });
    scene.typography.remap_labels(&slots);
    scene.label_series = std::mem::take(&mut scene.label_series)
        .into_iter()
        .filter_map(|(i, v)| slots.get(&i).map(|&new| (new, v)))
        .collect();
    scene.label_clips = std::mem::take(&mut scene.label_clips)
        .into_iter()
        .filter_map(|(i, v)| slots.get(&i).map(|&new| (new, v)))
        .collect();
}

fn retain_marks(scene: &mut Scene, keep: impl Fn(&str) -> bool) {
    let mut slots = BTreeMap::new();
    let mut removed = BTreeSet::new();
    scene.marks = std::mem::take(&mut scene.marks)
        .into_iter()
        .enumerate()
        .filter_map(|(i, mark)| {
            if !keep(&mark.id) {
                removed.insert(mark.id);
                return None;
            }
            slots.insert(i, slots.len());
            Some(mark)
        })
        .collect();
    scene.draw_order.retain_mut(|item| match item {
        DrawItem::Mark(i) => slots.get(i).is_some_and(|new| {
            *i = *new;
            true
        }),
        DrawItem::Label(_) => true,
    });
    for group in &mut scene.opacity_groups {
        group.members.retain(|id| !removed.contains(id));
    }
    scene.opacity_groups.retain(|g| !g.members.is_empty());
}

fn segment(mark: &Mark) -> Option<(GraphicPoint, GraphicPoint)> {
    let Geometry::Path(path) = &mark.geometry else {
        return None;
    };
    match path.as_slice() {
        [PathCommand::Move(a), PathCommand::Line(b)] => Some((*a, *b)),
        _ => None,
    }
}
fn add_x_grid(scene: &mut Scene) {
    let Some((a, b)) = scene
        .marks
        .iter()
        .find(|m| m.id == "axis-y")
        .and_then(segment)
    else {
        return;
    };
    let Some(template) = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("axis-y-grid-"))
    else {
        return;
    };
    let grid: Vec<_> = scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("axis-x-tick"))
        .filter_map(|tick| {
            let (point, _) = segment(tick)?;
            let mut mark = template.clone();
            mark.id = format!("showcase-grid-x:{}", tick.id);
            mark.geometry = Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(point.x, a.y)),
                PathCommand::Line(GraphicPoint::new(point.x, b.y)),
            ]);
            Some(mark)
        })
        .collect();
    let existing = scene.draw_items();
    let start = scene.marks.len();
    scene.marks.extend(grid);
    scene.draw_order = (start..scene.marks.len())
        .map(DrawItem::Mark)
        .chain(existing)
        .collect();
}

pub(super) fn apply(kind: &str, settings: &Settings, scene: &mut Scene) -> Result<()> {
    let x = settings.get("axis-x") == "hide";
    let y = settings.get("axis-y") == "hide";
    let single = settings.get("axis-visible") == "hide";
    let grid = settings.get("grid-visible") == "hide";
    let show_x_grid = settings.get("grid-x") == "show";
    if !x && !y && !single && !grid && !show_x_grid {
        return Ok(());
    }
    if show_x_grid {
        add_x_grid(scene);
    }
    let value_horizontal = scene
        .marks
        .iter()
        .find(|m| m.id == "axis-bar-value")
        .and_then(segment)
        .is_some_and(|(a, b)| a.y == b.y);
    retain_marks(scene, |id| {
        if id.starts_with("showcase-grid-x:") {
            return true;
        }
        // Match the layout's decoration namespace. Source IDs may contain "grid".
        let is_grid = match kind {
            "line" | "area" | "stream" | "scatter" => id.starts_with("axis-y-grid-"),
            "bar" | "boxplot" => id.starts_with("axis-bar-value-grid-"),
            "bump" => id.starts_with("bump-axis-") && id.contains("-grid:"),
            "area-bump" => id.starts_with("area-bump-grid:"),
            "radar" => id.starts_with("grid:"),
            "polar-bar" => id.starts_with("polar-grid-"),
            "geo-map" => id.starts_with("geo-line:graticule:"),
            _ => false,
        };
        if is_grid {
            return !grid;
        }
        if id.starts_with("axis-bar-value") {
            return !(if value_horizontal { x } else { y });
        }
        if id.starts_with("axis-x")
            || id.starts_with("bump-axis-top")
            || id.starts_with("bump-axis-bottom")
            || id.starts_with("area-bump-axis:")
            || id.starts_with("area-bump-tick:")
        {
            return !x;
        }
        if id.starts_with("axis-y") || id.starts_with("bump-axis-left") {
            return !y;
        }
        if id.starts_with("bullet-axis:")
            || id.starts_with("parallel-axis:")
            || id.starts_with("axis:")
        {
            return !single;
        }
        true
    });
    if x || y || single {
        let categories = super::cartesian::categories();
        let keep: Vec<_> = scene
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let owner = scene.label_series.get(&i);
                match kind {
                    // Bullet associates both its axis ticks and row title with the same row.
                    "bullet" => !single || owner.is_none_or(|row| row == &label.text),
                    // Marimekko interleaves numeric cell labels with category labels.
                    "marimekko" => !single || !categories.contains(&label.text),
                    _ if owner.is_some() => true,
                    "radar" | "polar-bar" | "parallel-coordinates" => !single,
                    _ if label.anchor == TextAnchor::Middle => !x,
                    _ => !y,
                }
            })
            .collect();
        retain_labels(scene, |i, _| keep[i]);
    }
    scene.validate()
}

pub(super) fn code(kind: &str, settings: &Settings) -> String {
    let selected: Vec<_> = controls(kind)
        .iter()
        .filter(|c| {
            let value = settings.get(c.key);
            !value.is_empty() && value != c.choices[0].0
        })
        .map(|c| format!("// {}: {}", c.label, settings.get(c.key)))
        .collect();
    if selected.is_empty() {
        return String::new();
    }
    format!(
        "{}\naxes_controls::apply(kind, &settings, &mut scene)?;",
        selected.join("\n")
    )
}
