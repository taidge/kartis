use kartis::browser::{BrowserChart, BrowserOptions};
use kartis::{ChartError, Result, TooltipOptions, TooltipPart, ValueFormat};

use crate::catalog::Settings;

fn is_presentation_key(key: &str) -> bool {
    matches!(
        key,
        "legend-display" | "tooltip-content" | "tooltip-format" | "tooltip-style"
    )
}

pub fn only_browser_changes(old: &Settings, new: &Settings) -> bool {
    old.0
        .iter()
        .filter(|(key, _)| !is_presentation_key(key))
        .eq(new.0.iter().filter(|(key, _)| !is_presentation_key(key)))
}

pub fn mount_options(settings: &Settings, options: &mut BrowserOptions) {
    options.tooltip = settings.get("tooltip-content") != "hidden";
    if settings.get("tooltip-content") == "value" {
        options.tooltip_options.lines = vec![vec![TooltipPart::Value]];
    }
    if settings.get("tooltip-format") == "fixed" {
        options.tooltip_options.value_format = ValueFormat::Fixed(2);
    }
}

pub fn set_stage(settings: &Settings) -> Result<()> {
    let stage = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|doc| doc.get_element_by_id("chart-stage"))
        .ok_or_else(|| ChartError("Chart stage is missing".into()))?;
    for (attr, key) in [
        ("data-legend-display", "legend-display"),
        ("data-tooltip-style", "tooltip-style"),
    ] {
        stage
            .set_attribute(attr, settings.get(key))
            .map_err(|e| ChartError(format!("Cannot style chart stage: {e:?}")))?;
    }
    Ok(())
}

pub fn update(chart: &BrowserChart, old: &Settings, new: &Settings) -> Result<()> {
    if ["tooltip-content", "tooltip-format"]
        .iter()
        .any(|key| old.get(key) != new.get(key))
    {
        let enabled = new.get("tooltip-content") != "hidden";
        let lines = if new.get("tooltip-content") == "value" {
            vec![vec![TooltipPart::Value]]
        } else {
            TooltipOptions::default().lines
        };
        let format = if new.get("tooltip-format") == "fixed" {
            ValueFormat::Fixed(2)
        } else {
            ValueFormat::Auto
        };
        chart.set_tooltip_content(enabled, lines, format)?;
    }
    if ["legend-display", "tooltip-style"]
        .iter()
        .any(|key| old.get(key) != new.get(key))
    {
        set_stage(new)?;
    }
    Ok(())
}
