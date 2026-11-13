# Summary domains, Bar index legends and legend actions

These options extend the shared Scene, SVG, Canvas and CPU rendering APIs. Existing defaults remain `SampleExtent`, `BarLegendSource::Keys`, `series_order: None` and static category legends.

| API | Meaning |
| --- | --- |
| `BoxPlotLayoutOptions.auto_domain_source` | Select `SampleExtent` or explicit `SummaryQuantiles`. |
| `BarStyleOptions.legend_source` | Select series `Keys` or category `Indexes`; `legend` remains the master switch. |
| `BarStyleOptions.series_order` | Optional complete source series-ID permutation. |
| `Scene.legend_roles` | Sparse `LegendRole::Series` / `Category` metadata keyed by real legend IDs. |
| `Chart::on_legend_activate` | Observe typed `LegendActivation { item, role }` intent; enable category action controls. |

`SampleExtent` preserves the previous BoxPlot sample-extrema domain, quantile arithmetic and total-order/ID sorting. `SummaryQuantiles` infers automatic bounds from all five configured summary values of visible boxes, including custom or repeated quantiles. Whisker policy, raw extrema and outliers do not supply this domain. Nullable observations remain absent rather than becoming zero. The mode sorts finite samples numerically, preserving input order for ties, including signed zero; the computed `source_ids` sequence follows that stable order without replacing any source identity.

Summary interpolation uses the original Nivo expression `x + (y - x) * t` when its difference and result are finite. An integral quantile index returns its original sample. This preserves ordinary narrow-domain rounding, constants and interpolated signed-zero behavior. Only a nonfinite intermediate difference or result falls back to the previous stable weighted expression. For example, the original expression overflows on some quantiles of `[-1e308, 1e308]`, while the finite fallback retains a usable summary. This is a deliberate finite extension. Selecting `SummaryQuantiles` also selects this statistical arithmetic when a complete domain or two fixed bounds bypass domain inference; it can change quantile bits from the default profile.

Independent `domain_bounds` replace only their fixed side after inference. A complete `domain` or two fixed bounds bypass inference, and supplying both `domain` and `domain_bounds` remains an error. Shared nice, reverse, round, clipping and scale validation keep their existing meanings. The existing finite empty-domain fallback and raw Log/inverse-Pow source checks remain: empty input does not reproduce Nivo's infinite empty bounds, and invalid raw zero/mixed-sign fallback inputs are not concealed by summary values.

For the configured UTC entry point, only automatic summary-domain endpoints use Date TimeClip: validate the finite Date range, truncate toward zero milliseconds and normalize zero to positive zero. Fixed endpoints still require bounded integral milliseconds. Original samples, fractional quantiles, medians and `map_statistic` queries are not truncated. Legacy nonconfigured UTC remains unsupported. A summary interval such as `0.1..0.9` therefore has domain `[0, 0]` while retaining fractional statistics. The reference Nivo time factory has a separate falsy-zero defect: literal zero endpoints can leave its default 2000 date interval. This profile keeps the mathematical `[0, 0]` domain instead of copying that defect.

Bar `series_order: Some(...)` must contain every source series ID exactly once, including hidden series. It drives grouped placement, stack/percent accumulation, the full series palette and Keys legend order; `None` retains lexical order. Hidden series retain their palette slots. `source_series` in ordinary and factory contexts remains the original input index. Category palettes keep their separate existing order, and explicit colors remain overrides.

An Indexes legend uses original categories and the first ordered nonhidden key's numeric palette color, even when that key's datum is null or zero. It is built before null, ink, value-domain crop and category-window filters, so an all-null category can still have a legend item without a fabricated bar. Horizontal index legends reverse once. A disabled legend, no visible key or zero category breadth produces no index items; emitted items retain the shared 256-item budget. Custom paints and hover do not replace the numeric legend palette. Keys keep the original series-control semantics, including neutral symbols for the existing category-color mode.

Roles distinguish category IDs from equal-looking series IDs. Missing role entries on existing items mean Series; `scene.legend_role(unknown)` returns `None`. Role keys must name real legend items and participate in Scene validation and equality. Category items do not follow the hidden-series set. Explicit `without_series`, hidden-state and series-toggle APIs retain their series meaning. Custom Bar layouts install roles before callbacks and freeze them with the accepted frame; subsequent role tampering is rejected by their binding and controlled-hover validation.

Without an action callback, category legends are static content. `Chart::on_legend_activate` makes them ordinary action controls while series keep their toggles. The activation carries the current raw item ID, label, color and role. Chart notifies intent before attempting a series toggle, and rechecks the current accepted role before toggling. Intent is not confirmation that hidden state committed. Controlled hidden state remains a caller proposal/acceptance contract. Custom legend renderers receive reactive `role`, `hidden` and `label`; their Widgets supply content rather than nested controls.

BrowserChart and RasterSurface require `legend_actions: true` in their mount options. Subscribing afterward does not convert static categories into buttons. Browser actions arrive as `BrowserEvent::LegendActivated`; raster actions use `on_legend_activate`. Queued hosts revalidate identity and role against the current accepted Scene. Category actions never implicitly hide series. The feature-gated helpers below show the opt-in at mount; the application supplies its root or scope, fonts and category behavior.

The complete program constructs both layouts and their controls. Its `web` helper is for wasm32 with the `web` feature; the `raster` helper requires `raster` and registered fonts covering the labels. Constructing the Chart builder does not simulate a user activation.

The example's `cfg(feature = ...)` names belong to the application package. With its `kartis` and `glory` dependencies already declared, forward those feature names and expose the browser helper's `web_sys::Element` type:

```toml
[features]
web = ["kartis/web", "dep:web-sys"]
raster = ["kartis/raster"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "=0.3.107", optional = true, features = ["Element"] }
```

```rust
use kartis::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

fn observations(values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(i.to_string(), "A", "", Some(*value)))
        .collect()
}

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 60.,
            bottom: 60.,
        },
        theme: Theme {
            colors: vec![Color(200, 40, 60, 255), Color(40, 80, 200, 255)],
            ..Default::default()
        },
        ..Default::default()
    };
    let source = observations(&[1., 2., 3., 4.]);
    let sample = boxplot_layout(&config, &source, &Default::default())?;
    let mut box_options = BoxPlotLayoutOptions {
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        orientation: StatisticsOrientation::Horizontal,
        ..Default::default()
    };
    let summary = boxplot_layout(&config, &source, &box_options)?;
    assert_eq!(sample.domain, [1., 4.]);
    assert_eq!(summary.domain, [1.3, 3.7]);
    assert_eq!(summary.boxes[0].extrema, [1., 4.]);
    assert_eq!(summary.boxes[0].quantiles, [1.3, 1.75, 2.5, 3.25, 3.7]);
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: Some(2.),
        max: None,
    });
    assert_eq!(
        boxplot_layout(&config, &source, &box_options)?.domain,
        [2., 3.7]
    );
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: None,
        max: Some(3.),
    });
    assert_eq!(
        boxplot_layout(&config, &source, &box_options)?.domain,
        [1.3, 3.]
    );

    box_options.domain_bounds = None;
    let utc = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let base = 8_000_000_000_000_000.;
    let dates = boxplot_layout_with_scale_options(
        &config,
        &observations(&[base, base + 5.]),
        &box_options,
        &utc,
    )?;
    assert_eq!(dates.value_scale.source_domain(), [base, base + 4.]);
    assert_eq!(dates.map_statistic(base + 4.)?, 420.);
    let sub_ms =
        boxplot_layout_with_scale_options(&config, &observations(&[0., 1.]), &box_options, &utc)?;
    assert_eq!(sub_ms.value_scale.source_domain(), [0., 0.]);
    assert_eq!(sub_ms.layout.boxes[0].quantiles[2], 0.5);
    assert_eq!(sub_ms.map_statistic(0.25)?, 240.);

    let categories = ["z", "zero", "missing"].map(str::to_string);
    let bars = [
        BarSeries::new("a", vec![Some(2.), None, None]),
        BarSeries::new("z", vec![None, Some(0.), None]),
    ];
    let bar_options = BarOptions {
        value_domain: Some([-1., 3.]),
        ..Default::default()
    };
    let style = BarStyleOptions {
        legend_source: BarLegendSource::Indexes,
        series_order: Some(vec!["z".into(), "a".into()]),
        ..Default::default()
    };
    let indexed = bar_layout_with_style(&config, &categories, &bars, &bar_options, &style)?;
    assert_eq!(
        indexed
            .scene
            .legend
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["z", "zero", "missing"]
    );
    assert!(
        indexed
            .scene
            .legend
            .iter()
            .all(|i| i.color == config.theme.colors[0])
    );
    assert_eq!(indexed.scene.legend_role("z"), Some(LegendRole::Category));
    assert_eq!(indexed.scene.legend_role("unknown"), None);
    let hidden_z = BTreeSet::from(["z".to_string()]);
    assert!(!indexed.scene.legend_hidden("z", &hidden_z));
    assert!(
        indexed
            .bars
            .iter()
            .any(|b| b.series == "z" && b.raw_value == 0. && b.source_series == 1)
    );
    assert!(!indexed.bars.iter().any(|b| b.category == "missing"));

    let hidden_options = BarOptions {
        hidden: hidden_z,
        ..bar_options.clone()
    };
    let hidden = bar_layout_with_style(&config, &categories, &bars, &hidden_options, &style)?;
    assert!(
        hidden
            .scene
            .legend
            .iter()
            .all(|i| i.color == config.theme.colors[1])
    );
    let keys_style = BarStyleOptions {
        legend_source: BarLegendSource::Keys,
        ..style.clone()
    };
    let keys = bar_layout_with_style(&config, &categories, &bars, &bar_options, &keys_style)?;
    assert_eq!(
        keys.scene
            .legend
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["z", "a"]
    );
    assert_eq!(keys.scene.legend_role("z"), Some(LegendRole::Series));

    let category_requests = Rc::new(RefCell::new(Vec::<String>::new()));
    let requests = category_requests.clone();
    let _controls = chart(indexed.scene.clone()).on_legend_activate(move |intent| {
        if intent.role == LegendRole::Category {
            requests.borrow_mut().push(intent.item.id);
        }
    });
    let _svg = render::to_svg(&summary.scene)?;
    let _canvas_script = render::to_canvas_script(&indexed.scene, "bars")?;
    Ok(())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn mount_browser_with_category_actions(
    root: web_sys::Element,
    scene: Scene,
    mut on_category: impl FnMut(String) + 'static,
) -> kartis::Result<kartis::browser::BrowserChart> {
    use kartis::browser::{BrowserChart, BrowserEvent, BrowserOptions};
    let host = BrowserChart::mount(
        root,
        scene,
        BrowserOptions {
            legend_actions: true,
            ..Default::default()
        },
    )?;
    host.on_event(move |event| {
        if let BrowserEvent::LegendActivated(intent) = event
            && intent.role == LegendRole::Category
        {
            on_category(intent.item.id);
        }
    });
    Ok(host)
}

#[cfg(feature = "raster")]
pub fn mount_raster_with_category_actions(
    scope: &glory::Scope,
    scene: Scene,
    fonts: render::raster::FontRegistry,
    mut on_category: impl FnMut(String) + 'static,
) -> kartis::Result<RasterSurface> {
    let host = RasterSurface::mount(
        scope,
        scene,
        fonts,
        RasterSurfaceOptions {
            legend_actions: true,
            ..Default::default()
        },
    )?;
    host.on_legend_activate(move |intent| {
        if intent.role == LegendRole::Category {
            on_category(intent.item.id);
        }
    })?;
    Ok(host)
}
```

See [independent bounds](endpoint-point-bounds.md), [typed Bar factories](typed-labels.md) and [feature scope](feature-gaps.md) for related contracts. This guide describes API behavior; it does not claim completed browser, host-runtime or full test acceptance.
