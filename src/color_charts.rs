//! Shared data-color scales consumed by geographic and calendar layouts.
//! Original entry points keep their earlier profiles and palettes.
use std::collections::BTreeMap;

use crate::*;
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMappedGeoLayout {
    pub scene: Scene,
    pub projection: GeoProjectionContext,
    pub color_scale: ColorScale,
}
/// All supplied feature values establish the scale before projection clipping.
/// Missing feature values use scale.unknown, including when map.fill is set.
pub fn choropleth_with_color_scale(
    c: &ChartConfig,
    features: &[GeoFeature],
    map: &GeoMapOptions,
    colors: &ColorScaleOptions,
) -> Result<ColorMappedGeoLayout> {
    choropleth_with_color_profile(c, features, map, colors, ColorInterpolation::Srgb)
}

/// Use the selected color space across geographic ink and legend stops.
pub fn choropleth_with_color_profile(
    c: &ChartConfig,
    features: &[GeoFeature],
    map: &GeoMapOptions,
    colors: &ColorScaleOptions,
    profile: impl Into<ColorScaleProfile>,
) -> Result<ColorMappedGeoLayout> {
    let scale = colors.resolve_with_profile(features.iter().map(|f| f.value), profile)?;
    let mut layout = geographic_map_layout(c, features, map)?;
    let values = features
        .iter()
        .map(|f| (f.id.as_str(), f.value))
        .collect::<BTreeMap<_, _>>();
    for mark in &mut layout.scene.marks {
        if let Some(value) = values.get(mark.series.as_str()) {
            mark.fill = Some(scale.map(*value));
        }
    }
    layout.scene.validate()?;
    Ok(ColorMappedGeoLayout {
        scene: layout.scene,
        projection: layout.projection,
        color_scale: scale,
    })
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMappedCalendarLayout {
    pub scene: Scene,
    pub color_scale: ColorScale,
}
/// Calendar cells without a supplied value are colored as missing, rather
/// than confusing the Scene's finite scalar placeholder with an observed zero.
pub fn calendar_range_with_color_scale(
    c: &ChartConfig,
    data: &[CalendarDate],
    layout: &CalendarRangeOptions,
    colors: &ColorScaleOptions,
) -> Result<ColorMappedCalendarLayout> {
    calendar_range_with_color_profile(c, data, layout, colors, ColorInterpolation::Srgb)
}

/// Use the selected color space in the complete-year calendar profile.
pub fn calendar_range_with_color_profile(
    c: &ChartConfig,
    data: &[CalendarDate],
    layout: &CalendarRangeOptions,
    colors: &ColorScaleOptions,
    profile: impl Into<ColorScaleProfile>,
) -> Result<ColorMappedCalendarLayout> {
    let mut scene = calendar_range(c, data, layout)?;
    let color_scale = colors.resolve_with_profile(data.iter().map(|d| Some(d.value)), profile)?;
    let values = data
        .iter()
        .map(|d| {
            (
                format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
                d.value,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for mark in &mut scene.marks {
        mark.fill = Some(color_scale.map(values.get(&mark.id).copied()));
    }
    scene.validate()?;
    Ok(ColorMappedCalendarLayout { scene, color_scale })
}
