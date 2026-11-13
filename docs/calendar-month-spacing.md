# Calendar month spacing

[简体中文](calendar-month-spacing.zh-CN.md)

`CalendarLayoutOptions::month_spacing` separates successive months along the week axis. It defaults to zero and keeps existing geometry, metadata, Debug and exports. Positive values are supported only by `FullYears`; `Compact`, which corresponds to Nivo TimeRange, rejects them. Values must be finite, nonnegative and at most 1024 logical pixels. The existing area and resource checks still apply: a syntactically valid gap can leave no drawable cell area and reject.

The width allocation reserves twelve spacings, including a final unused allowance, while a date moves by `(month-1)*spacing`. Horizontal calendars shift X, vertical calendars shift Y. Cells, actual month union paths, month bounds, day labels and month labels share that origin. `month_outline_padding` remains an independent outward expansion of the month boundary; it does not move dates.

The program creates four real Memory charts and standalone SVG/Canvas sources: default zero and explicit 8px in both orientations. With `raster`, it also renders four CPU PNGs using the supplied tracked Lato face. Pass an output directory to export the files. The generated HTML waits for that font before its first Canvas draw and publishes `window.calendarMonthSpacingReady` after all four original scripts execute.

Run `cargo run --example calendar_month_spacing`. To export CPU output, use `cargo run --features raster --example calendar_month_spacing -- <output-directory>`; obtain the project's normal output directory through `cargo metadata --no-deps --format-version 1`. The example embeds the supplied fixture font at compile time.

```rust
use std::{fs, path::PathBuf};
#[cfg(feature = "raster")]
use kartis::render::raster::{FaceDescriptor, FontRegistry, RasterOptions, render_raster};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let data = [
        CalendarCell::new((2024, 1, 31), Some(0.)),
        CalendarCell::new((2024, 2, 1), Some(7.)),
        CalendarCell::new((2024, 2, 29), None),
    ];
    let options = CalendarLayoutOptions {
        mode: CalendarLayoutMode::FullYears,
        start: Some((2024, 1, 1)),
        end: Some((2024, 12, 31)),
        day_spacing: 0.,
        weekday_space: 0.,
        weekday_ticks: vec![],
        year_labels: Some(true),
        month_labels: true,
        month_style: CalendarTextStyle {
            size: Some(11.),
            ..Default::default()
        },
        colors: ColorScaleOptions {
            domain: Some([0., 7.]),
            colors: vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)],
            unknown: Color(210, 215, 220, 255),
            ..Default::default()
        },
        ..Default::default()
    };
    let output = std::env::args().nth(1).map(PathBuf::from);
    if let Some(dir) = &output {
        fs::create_dir_all(dir)?;
    }
    #[cfg(feature = "raster")]
    let fonts = {
        let mut fonts = FontRegistry::new();
        let face = fonts.register(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fonts/Lato-Regular.ttf"
            ))
            .as_slice(),
            0,
        )?;
        fonts.bind_face(
            face,
            FaceDescriptor {
                family: FontFamily::Named("Lato".into()),
                weight: 400.,
            },
        )?;
        fonts
    };
    let mut scene_facts = vec![];
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><style>@font-face{font-family:Lato;src:url(Lato-Regular.ttf)}body{font-family:Lato;background:white}article{display:inline-block;vertical-align:top}canvas{display:block}</style><h1>Calendar month spacing</h1>",
    );
    for (name, orientation, spacing) in [
        ("horizontal-default", CalendarOrientation::Horizontal, 0.),
        ("horizontal-eight", CalendarOrientation::Horizontal, 8.),
        ("vertical-default", CalendarOrientation::Vertical, 0.),
        ("vertical-eight", CalendarOrientation::Vertical, 8.),
    ] {
        let vertical = orientation == CalendarOrientation::Vertical;
        let mut config = ChartConfig {
            width: if vertical { 220. } else { 800. },
            height: if vertical { 800. } else { 220. },
            ..Default::default()
        };
        config.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
        let layout = calendar_layout(
            &config,
            &data,
            &CalendarLayoutOptions {
                orientation,
                month_spacing: spacing,
                ..options.clone()
            },
        )?;
        let january = layout.day((2024, 1, 31)).unwrap();
        let february = layout.day((2024, 2, 1)).unwrap();
        assert_eq!(january.week, february.week);
        let difference = if vertical {
            february.rect.y - january.rect.y
        } else {
            february.rect.x - january.rect.x
        };
        assert!((difference - spacing).abs() < 1e-9);
        assert_eq!(january.value, Some(0.));
        assert_eq!(february.value, Some(7.));
        let leap = layout.day((2024, 2, 29)).unwrap();
        let hole = layout.day((2024, 3, 1)).unwrap();
        assert!(leap.provided && leap.value.is_none());
        assert!(!hole.provided && hole.value.is_none());
        let r = february.rect;
        assert_eq!(
            layout
                .hit_test(r.x + r.width / 2., r.y + r.height / 2.)
                .unwrap()
                .id,
            "2024-02-01",
        );
        let app = glory::App::memory();
        app.mount(chart(layout.scene.clone()))?;
        app.flush()?;
        let svg = render::to_svg(&layout.scene)?;
        let canvas_id = format!("canvas-{name}");
        let canvas = render::to_canvas_script(&layout.scene, &canvas_id)?;
        assert!(svg.contains("<svg") && !canvas.is_empty());
        if let Some(dir) = &output {
            fs::write(dir.join(format!("{name}.svg")), &svg)?;
            fs::write(dir.join(format!("{name}.js")), &canvas)?;
            #[cfg(feature = "raster")]
            {
                let image = render_raster(&layout.scene, &fonts, &RasterOptions::default())?;
                fs::write(dir.join(format!("{name}.png")), image.png()?)?;
            }
        }
        let days = [january, february, leap, hole]
            .iter()
            .map(|d| {
                let value = d.value.map_or_else(|| "null".into(), |v| v.to_string());
                format!(
                    "{{\"id\":\"{}\",\"value\":{},\"provided\":{},\"rect\":[{},{},{},{}]}}",
                    d.id, value, d.provided, d.rect.x, d.rect.y, d.rect.width, d.rect.height
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        scene_facts.push(format!("{{\"name\":\"{name}\",\"vertical\":{vertical},\"spacing\":{spacing},\"width\":{},\"height\":{},\"days\":[{days}]}}", config.width, config.height));
        html.push_str(&format!("<article data-case=\"{name}\"><h2>{name}</h2>{svg}<canvas id=\"{canvas_id}\" width=\"{}\" height=\"{}\"></canvas></article>", config.width, config.height));
    }
    let mut rejections = vec![];
    for (name, mode, spacing) in [
        ("compact-positive", CalendarLayoutMode::Compact, 8.),
        ("negative", CalendarLayoutMode::FullYears, -1.),
        ("nan", CalendarLayoutMode::FullYears, f64::NAN),
        ("infinite", CalendarLayoutMode::FullYears, f64::INFINITY),
        ("no-cell-area", CalendarLayoutMode::FullYears, 100.),
    ] {
        let error = calendar_layout(
            &ChartConfig {
                width: 800.,
                height: 220.,
                ..Default::default()
            },
            &data,
            &CalendarLayoutOptions {
                mode,
                month_spacing: spacing,
                ..options.clone()
            },
        )
        .unwrap_err();
        rejections.push(format!("{{\"case\":\"{name}\",\"error\":{:?}}}", error.0));
    }
    if let Some(dir) = output {
        html.push_str("<script type=\"module\">await document.fonts.load('13px Lato');await document.fonts.ready;for(const name of ['horizontal-default','horizontal-eight','vertical-default','vertical-eight']){const response=await fetch(name+'.js');if(!response.ok)throw Error('Canvas source failed: '+name);new Function(await response.text())();}window.calendarMonthSpacingMetadata=await(await fetch('metadata.json')).json();window.calendarMonthSpacingReady=true;</script>");
        fs::write(
            dir.join("Lato-Regular.ttf"),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fonts/Lato-Regular.ttf"
            )),
        )?;
        fs::write(dir.join("index.html"), html)?;
        fs::write(
            dir.join("metadata.json"),
            format!(
                "{{\"schema\":\"calendar-month-spacing-example-v1\",\"scenes\":[{}],\"rejections\":[{}]}}\n",
                scene_facts.join(","),
                rejections.join(",")
            ),
        )?;
    }
    println!("4 real Memory/SVG/Canvas scenes and 5 actual builder rejections passed");
    Ok(())
}
```

`day()` preserves the source date, nullable value and `provided` distinction: an explicit `None` and an absent day are both missing values but remain distinguishable. A real zero stays zero. `hit_test()` follows the shifted day geometry. Existing Scene-only event scalar semantics remain unchanged; use Calendar metadata when the missing/provided distinction matters.

The original oracle executes Nivo 0.99's complete `computeLayout`, data binding and month/year legend-position functions with actual D3-time and Lodash. It retains the original Sunday-Jan1 extra reserved week. Kartis's existing layout uses occupied week columns instead. Thus this feature is not a promise of global prop-for-prop Nivo layout equality. For the authored height-limited 10px reference profiles, eight cases match native raw origins and the two 2023 multi-year cases retain the independently computed 5px origin difference (native 54 versus occupied 53 columns). All raw native records remain available in the fixture.

Existing year-label header placement and weekday-label anchors remain Kartis's policy. Weekday labels stay next to the year panel; they are not repeated at every month. Month labels use the shifted actual month bounds; their rotation and baseline policies are unchanged. Arbitrary first weekdays, rectangular cells, outlines with padding and zoned aggregation remain Rust extensions, tested separately from the original Sunday/square reference subset.

See the [verification record](verification.md) for reference profiles and acceptance details.
