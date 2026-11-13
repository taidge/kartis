//! Compact inclusive TimeRange and styled full-year civil calendars.
//! Nivo reference: packages/calendar/src/{compute/timeRange,types,props}.ts.
//! Dates use the existing Gregorian and explicit-zone calendar authorities.
use crate::{scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::collections::{BTreeMap, BTreeSet};
pub type CalendarCivilDate = (i32, u8, u8);
pub const MAX_CALENDAR_LAYOUT_DAYS: usize = 12_000;
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarCell {
    pub date: CalendarCivilDate,
    pub value: Option<f64>,
}
impl CalendarCell {
    pub fn new(date: CalendarCivilDate, value: Option<f64>) -> Self {
        Self { date, value }
    }
}
impl From<&CalendarDate> for CalendarCell {
    fn from(d: &CalendarDate) -> Self {
        Self::new((d.year, d.month, d.day), Some(d.value))
    }
}
impl From<CalendarDate> for CalendarCell {
    fn from(d: CalendarDate) -> Self {
        Self::from(&d)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarLayoutMode {
    #[default]
    Compact,
    FullYears,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarWeekday {
    #[default]
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}
impl CalendarWeekday {
    pub fn index(self) -> usize {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarLegendPosition {
    #[default]
    Before,
    After,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarDayLabel {
    #[default]
    None,
    Value,
    DayOfMonth,
    IsoDate,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalendarTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
}
impl CalendarTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && self.rotation.is_finite()
                && (-360. ..=360.).contains(&self.rotation),
            "invalid calendar text style",
        )
    }
    fn size(&self, c: &ChartConfig) -> f64 {
        self.size.unwrap_or(c.theme.font_size)
    }
    fn color(&self, c: &ChartConfig) -> Color {
        self.color.unwrap_or(c.theme.text)
    }
}
#[derive(Clone, PartialEq)]
pub struct CalendarLayoutOptions {
    pub mode: CalendarLayoutMode,
    pub orientation: CalendarOrientation,
    /// Inclusive local civil dates. None/None infers actual first/last input
    /// dates. FullYears expands either resolved interval to complete years.
    pub start: Option<CalendarCivilDate>,
    pub end: Option<CalendarCivilDate>,
    pub first_weekday: CalendarWeekday,
    /// Positions in the week shifted by first_weekday, each in 0..=6.
    pub weekday_ticks: Vec<u8>,
    pub weekdays: [String; 7],
    pub weekday_space: f64,
    pub weekday_offset: f64,
    pub weekday_style: CalendarTextStyle,
    pub square: bool,
    pub day_spacing: f64,
    pub day_radius: f64,
    /// Bordered days use Scene's existing half-stroke-width plus 2px picking
    /// allowance. Fill-only days use their actual rounded geometry.
    pub day_border: Option<GraphicStroke>,
    pub day_opacity: f64,
    pub day_labels: CalendarDayLabel,
    pub day_label_style: CalendarTextStyle,
    pub day_label_offset: [f64; 2],
    pub value_format: axes::ValueFormat,
    pub empty_label: String,
    pub colors: ColorScaleOptions,
    pub month_border: Option<GraphicStroke>,
    /// Outward offset of the actual month polyomino boundary. At most one
    /// quarter of the smallest cell pitch. The expanded tile union preserves
    /// actual concave boundaries and merges touching components correctly.
    pub month_outline_padding: f64,
    /// Extra separation along the week axis per zero-based month. Zero preserves
    /// the legacy layout. Positive spacing is supported only by FullYears;
    /// Compact (the TimeRange model) rejects it. Cell sizing reserves twelve
    /// spacings, matching the original Calendar allocation.
    pub month_spacing: f64,
    pub month_labels: bool,
    pub months: [String; 12],
    pub month_names: BTreeMap<(i32, u8), String>,
    pub month_label_position: CalendarLegendPosition,
    pub month_label_offset: f64,
    pub month_style: CalendarTextStyle,
    /// None displays years only in FullYears mode.
    pub year_labels: Option<bool>,
    pub year_style: CalendarTextStyle,
    pub year_gap: f64,
    pub year_legend: bool,
    /// Days and their owned labels are omitted; shared month/year/weekday
    /// decoration stays. Panel/date domains retain their accepted positions.
    pub hidden_years: BTreeSet<i32>,
}
impl std::fmt::Debug for CalendarLayoutOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = f.debug_struct("CalendarLayoutOptions");
        output.field("mode", &self.mode);
        output.field("orientation", &self.orientation);
        output.field("start", &self.start);
        output.field("end", &self.end);
        output.field("first_weekday", &self.first_weekday);
        output.field("weekday_ticks", &self.weekday_ticks);
        output.field("weekdays", &self.weekdays);
        output.field("weekday_space", &self.weekday_space);
        output.field("weekday_offset", &self.weekday_offset);
        output.field("weekday_style", &self.weekday_style);
        output.field("square", &self.square);
        output.field("day_spacing", &self.day_spacing);
        output.field("day_radius", &self.day_radius);
        output.field("day_border", &self.day_border);
        output.field("day_opacity", &self.day_opacity);
        output.field("day_labels", &self.day_labels);
        output.field("day_label_style", &self.day_label_style);
        output.field("day_label_offset", &self.day_label_offset);
        output.field("value_format", &self.value_format);
        output.field("empty_label", &self.empty_label);
        output.field("colors", &self.colors);
        output.field("month_border", &self.month_border);
        output.field("month_outline_padding", &self.month_outline_padding);
        if self.month_spacing != 0. {
            output.field("month_spacing", &self.month_spacing);
        }
        output.field("month_labels", &self.month_labels);
        output.field("months", &self.months);
        output.field("month_names", &self.month_names);
        output.field("month_label_position", &self.month_label_position);
        output.field("month_label_offset", &self.month_label_offset);
        output.field("month_style", &self.month_style);
        output.field("year_labels", &self.year_labels);
        output.field("year_style", &self.year_style);
        output.field("year_gap", &self.year_gap);
        output.field("year_legend", &self.year_legend);
        output.field("hidden_years", &self.hidden_years);
        output.finish()
    }
}

impl Default for CalendarLayoutOptions {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            orientation: CalendarOrientation::Horizontal,
            start: None,
            end: None,
            first_weekday: Default::default(),
            weekday_ticks: vec![1, 3, 5],
            weekdays: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_owned),
            weekday_space: 40.,
            weekday_offset: 8.,
            weekday_style: Default::default(),
            square: true,
            day_spacing: 2.,
            day_radius: 0.,
            day_border: Some(GraphicStroke {
                color: Color(255, 255, 255, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            day_opacity: 1.,
            day_labels: Default::default(),
            day_label_style: Default::default(),
            day_label_offset: [0., 0.],
            value_format: Default::default(),
            empty_label: "missing".into(),
            colors: Default::default(),
            month_border: Some(GraphicStroke {
                color: Color(70, 80, 100, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            month_outline_padding: 0.,
            month_spacing: 0.,
            month_labels: true,
            months: [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .map(str::to_owned),
            month_names: BTreeMap::new(),
            month_label_position: Default::default(),
            month_label_offset: 10.,
            month_style: Default::default(),
            year_labels: None,
            year_style: Default::default(),
            year_gap: 24.,
            year_legend: false,
            hidden_years: BTreeSet::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedCalendarDay {
    pub date: CalendarCivilDate,
    pub id: String,
    pub value: Option<f64>,
    pub provided: bool,
    pub formatted_value: Option<String>,
    pub label: String,
    pub rect: ClipRect,
    pub geometry: Geometry,
    pub week: usize,
    pub weekday: usize,
    pub year: i32,
    pub month: u8,
    pub color: Color,
    pub visible: bool,
}
impl ComputedCalendarDay {
    pub fn tooltip_datum(&self) -> TooltipDatum<'_> {
        TooltipDatum {
            id: &self.id,
            label: &self.id,
            series: "",
            value: self.value,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedCalendarMonth {
    pub year: i32,
    pub month: u8,
    pub day_ids: Vec<String>,
    pub outline: Geometry,
    pub bounds: ClipRect,
    pub mark_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarPanel {
    pub year: Option<i32>,
    pub weeks: usize,
    pub grid: ClipRect,
    pub cell_width: f64,
    pub cell_height: f64,
}
#[derive(Clone, Debug)]
pub struct CalendarLayout {
    pub scene: Scene,
    pub days: Vec<ComputedCalendarDay>,
    pub months: Vec<ComputedCalendarMonth>,
    pub panels: Vec<CalendarPanel>,
    pub start: CalendarCivilDate,
    pub end: CalendarCivilDate,
    pub color_scale: ColorScale,
}
impl CalendarLayout {
    pub fn day(&self, date: CalendarCivilDate) -> Option<&ComputedCalendarDay> {
        self.days
            .binary_search_by_key(&date, |d| d.date)
            .ok()
            .and_then(|i| self.days.get(i))
    }
    /// Data-only picking: shared month outlines do not intercept days. The
    /// usual Scene near-stroke allowance applies when a day has a border.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&ComputedCalendarDay> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || y < 0.
            || x > self.scene.width
            || y > self.scene.height
        {
            return None;
        }
        self.scene
            .paint_marks()
            .rev()
            .filter(|m| !m.label.is_empty() && m.opacity > 0. && crate::scene::contains(m, x, y))
            .find_map(|m| {
                self.days
                    .binary_search_by(|d| d.id.cmp(&m.id))
                    .ok()
                    .and_then(|i| self.days.get(i))
                    .filter(|d| d.visible)
            })
    }
}
fn serial(date: CalendarCivilDate) -> Result<i64> {
    let (y, m, d) = date;
    ensure(
        (1..=9999).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d),
        "invalid calendar civil date",
    )?;
    let day = crate::time::days_from_civil(i64::from(y), i64::from(m), i64::from(d));
    ensure(
        crate::time::civil_from_days(day) == (i64::from(y), i64::from(m), i64::from(d)),
        "invalid calendar Gregorian date",
    )?;
    Ok(day)
}
fn civil(day: i64) -> CalendarCivilDate {
    let (y, m, d) = crate::time::civil_from_days(day);
    (y as i32, m as u8, d as u8)
}
fn iso(date: CalendarCivilDate) -> String {
    format!("{:04}-{:02}-{:02}", date.0, date.1, date.2)
}
fn weekday(day: i64, first: CalendarWeekday) -> usize {
    (day + 4 - first.index() as i64).rem_euclid(7) as usize
}
fn stroke(s: Option<GraphicStroke>) -> Result<()> {
    ensure(
        s.is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width)),
        "invalid calendar border",
    )
}
fn validate(o: &CalendarLayoutOptions) -> Result<()> {
    for t in [
        &o.weekday_style,
        &o.day_label_style,
        &o.month_style,
        &o.year_style,
    ] {
        t.validate()?;
    }
    stroke(o.day_border)?;
    stroke(o.month_border)?;
    o.value_format.validate()?;
    ensure(
        [
            o.weekday_space,
            o.weekday_offset,
            o.day_spacing,
            o.day_radius,
            o.month_outline_padding,
            o.month_spacing,
            o.month_label_offset,
            o.year_gap,
        ]
        .into_iter()
        .all(|v| v.is_finite() && (0. ..=1024.).contains(&v))
            && o.day_label_offset
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 16384.)
            && o.day_opacity.is_finite()
            && (0. ..=1.).contains(&o.day_opacity),
        "invalid calendar dimensions or opacity",
    )?;
    ensure(
        o.month_spacing == 0. || o.mode == CalendarLayoutMode::FullYears,
        "calendar month spacing requires FullYears mode",
    )?;
    ensure(
        o.weekday_ticks.len() <= 7
            && o.weekday_ticks.iter().all(|v| *v < 7)
            && o.weekday_ticks.iter().collect::<BTreeSet<_>>().len() == o.weekday_ticks.len(),
        "calendar weekday ticks must be distinct shifted indices in 0..=6",
    )?;
    ensure(
        o.weekdays
            .iter()
            .chain(o.months.iter())
            .all(|s| s.len() <= 128)
            && o.empty_label.len() <= 512
            && o.month_names.len() <= 384
            && o.month_names.iter().all(|((y, m), text)| {
                (1..=9999).contains(y) && (1..=12).contains(m) && text.len() <= 512
            }),
        "calendar display label budget exceeded",
    )
}
fn range(data: &[CalendarCell], o: &CalendarLayoutOptions) -> Result<(i64, i64)> {
    let (start, end) = match (o.start, o.end) {
        (Some(a), Some(b)) => (serial(a)?, serial(b)?),
        (None, None) => {
            let a =
                data.iter().map(|d| d.date).min().ok_or_else(|| {
                    ChartError("empty calendar needs an explicit date range".into())
                })?;
            let b = data.iter().map(|d| d.date).max().unwrap();
            (serial(a)?, serial(b)?)
        }
        _ => {
            return Err(ChartError(
                "calendar requires both inclusive endpoints or neither".into(),
            ));
        }
    };
    ensure(
        start <= end && civil(end).0 - civil(start).0 < 32,
        "calendar range must ascend and span at most 32 years",
    )?;
    let (start, end) = if o.mode == CalendarLayoutMode::FullYears {
        (
            serial((civil(start).0, 1, 1))?,
            serial((civil(end).0, 12, 31))?,
        )
    } else {
        (start, end)
    };
    ensure(
        end - start < MAX_CALENDAR_LAYOUT_DAYS as i64,
        "calendar layout exceeds 12000 days",
    )?;
    Ok((start, end))
}
fn transpose_geometry(g: Geometry, vertical: bool) -> Geometry {
    if !vertical {
        return g;
    }
    let p = |p: P| P::new(p.y, p.x);
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Geometry::Rect {
            x: y,
            y: x,
            width: height,
            height: width,
        },
        Geometry::Circle { x, y, radius } => Geometry::Circle { x: y, y: x, radius },
        Geometry::Path(cmds) => Geometry::Path(
            cmds.into_iter()
                .map(|c| match c {
                    C::Move(a) => C::Move(p(a)),
                    C::Line(a) => C::Line(p(a)),
                    C::Cubic { first, second, end } => C::Cubic {
                        first: p(first),
                        second: p(second),
                        end: p(end),
                    },
                    C::Close => C::Close,
                })
                .collect(),
        ),
    }
}
fn rect(r: ClipRect, vertical: bool) -> ClipRect {
    if vertical {
        ClipRect {
            x: r.y,
            y: r.x,
            width: r.height,
            height: r.width,
        }
    } else {
        r
    }
}
fn point(p: [f64; 2], vertical: bool) -> [f64; 2] {
    if vertical { [p[1], p[0]] } else { p }
}
fn text(
    scene: &mut Scene,
    c: &ChartConfig,
    p: [f64; 2],
    value: String,
    anchor: TextAnchor,
    style: &CalendarTextStyle,
    owner: Option<String>,
) {
    let r = style.rotation.to_radians();
    let offset = style.size(c) / 3.;
    if let Some(owner) = owner {
        scene.label_series.insert(scene.labels.len(), owner);
    }
    scene.labels.push(Label {
        x: p[0] - r.sin() * offset,
        y: p[1] + r.cos() * offset,
        text: value,
        color: style.color(c),
        size: style.size(c),
        anchor,
        rotation: style.rotation,
    });
}

/// Lay out a compact inclusive civil interval, or styled complete years. The
/// source is nullable; absent requested dates are synthesized as missing days.
/// Explicit data outside the accepted interval and duplicate dates reject.
pub fn calendar_layout(
    c: &ChartConfig,
    data: &[CalendarCell],
    o: &CalendarLayoutOptions,
) -> Result<CalendarLayout> {
    calendar_layout_with_color_profile(c, data, o, crate::ColorInterpolation::Srgb)
}

/// Use the selected color space for day ink and the resolved legend scale.
pub fn calendar_layout_with_color_profile(
    c: &ChartConfig,
    data: &[CalendarCell],
    o: &CalendarLayoutOptions,
    profile: impl Into<crate::ColorScaleProfile>,
) -> Result<CalendarLayout> {
    c.validate()?;
    validate(o)?;
    ensure(
        data.len() <= MAX_CALENDAR_LAYOUT_DAYS,
        "calendar input exceeds 12000 dates",
    )?;
    let mut values = BTreeMap::new();
    for d in data {
        let key = serial(d.date)?;
        ensure(
            d.value.is_none_or(f64::is_finite),
            "calendar values must be finite or null",
        )?;
        ensure(
            values.insert(key, d.value).is_none(),
            "duplicate calendar date",
        )?;
    }
    let (start, end) = range(data, o)?;
    ensure(
        values.keys().all(|d| *d >= start && *d <= end),
        "calendar datum lies outside the date interval",
    )?;
    let first_year = civil(start).0;
    let last_year = civil(end).0;
    ensure(
        o.hidden_years.len() < 33
            && o.hidden_years
                .iter()
                .all(|y| *y >= first_year && *y <= last_year),
        "calendar hidden year lies outside interval",
    )?;
    let scale = o.colors.resolve_with_profile(
        values
            .iter()
            .filter(|(d, _)| !o.hidden_years.contains(&civil(**d).0))
            .map(|(_, v)| *v),
        profile,
    )?;
    let vertical = o.orientation == CalendarOrientation::Vertical;
    let mut canonical = c.clone();
    if vertical {
        std::mem::swap(&mut canonical.width, &mut canonical.height);
        canonical.margin = Margin {
            left: c.margin.top,
            right: c.margin.bottom,
            top: c.margin.left,
            bottom: c.margin.right,
        };
    }
    let ranges: Vec<_> = if o.mode == CalendarLayoutMode::FullYears {
        (first_year..=last_year)
            .map(|y| {
                (
                    serial((y, 1, 1)).unwrap(),
                    serial((y, 12, 31)).unwrap(),
                    Some(y),
                )
            })
            .collect()
    } else {
        vec![(start, end, None)]
    };
    let show_years = o
        .year_labels
        .unwrap_or(o.mode == CalendarLayoutMode::FullYears);
    let slot = (canonical.inner_height() - o.year_gap * ranges.len().saturating_sub(1) as f64)
        / ranges.len() as f64;
    let month_head = if o.month_labels {
        o.month_style.size(c) + o.month_label_offset
    } else {
        0.
    };
    let year_head = if show_years {
        o.year_style.size(c) * 1.8
    } else {
        0.
    };
    let before = o.month_label_position == CalendarLegendPosition::Before;
    let header = year_head + if before { month_head } else { 0. };
    let footer = if before { 0. } else { month_head };
    let max_weeks = ranges
        .iter()
        .map(|(a, b, _)| ((b - a + weekday(*a, o.first_weekday) as i64) / 7 + 1) as usize)
        .max()
        .unwrap();
    let available_width = canonical.inner_width() - o.weekday_space;
    let mut scene = c.scene()?;
    let mut days = vec![];
    let mut months = vec![];
    let mut panels = vec![];
    for (index, (a, b, year)) in ranges.into_iter().enumerate() {
        let leading = weekday(a, o.first_weekday);
        let weeks = ((b - a + leading as i64) / 7 + 1) as usize;
        let columns = if o.mode == CalendarLayoutMode::FullYears {
            max_weeks
        } else {
            weeks
        };
        let available_height = slot - header - footer;
        let month_space = 12. * o.month_spacing;
        let mut cw =
            (available_width - o.day_spacing * (columns + 1) as f64 - month_space) / columns as f64;
        let mut ch = (available_height - o.day_spacing * 8.) / 7.;
        ensure(
            cw.is_finite() && ch.is_finite() && cw > 0. && ch > 0.,
            "calendar spacing and labels leave no cell area",
        )?;
        if o.square {
            let size = cw.min(ch);
            cw = size;
            ch = size;
        }
        let w = cw * columns as f64 + o.day_spacing * (columns + 1) as f64 + month_space;
        let h = ch * 7. + o.day_spacing * 8.;
        let left = canonical.margin.left + o.weekday_space + (available_width - w) / 2.;
        let top = canonical.margin.top
            + index as f64 * (slot + o.year_gap)
            + header
            + (available_height - h) / 2.;
        ensure(
            o.month_outline_padding <= (cw + o.day_spacing).min(ch + o.day_spacing) / 4.,
            "calendar month outline padding exceeds one quarter cell pitch",
        )?;
        let grid = ClipRect {
            x: left,
            y: top,
            width: w,
            height: h,
        };
        panels.push(CalendarPanel {
            year,
            weeks,
            grid: rect(grid, vertical),
            cell_width: if vertical { ch } else { cw },
            cell_height: if vertical { cw } else { ch },
        });
        let mut month_cells = BTreeMap::<(i32, u8), Vec<(usize, usize, String)>>::new();
        for day in a..=b {
            let date = civil(day);
            let week = ((day - a + leading as i64) / 7) as usize;
            let wd = weekday(day, o.first_weekday);
            let id = iso(date);
            let value = values.get(&day).copied().flatten();
            let color = scale.map(value);
            let provided = values.contains_key(&day);
            let r = ClipRect {
                x: left
                    + o.day_spacing
                    + week as f64 * (cw + o.day_spacing)
                    + f64::from(date.1 - 1) * o.month_spacing,
                y: top + o.day_spacing + wd as f64 * (ch + o.day_spacing),
                width: cw,
                height: ch,
            };
            let geometry = transpose_geometry(
                crate::cartesian::round_bar(
                    Geometry::Rect {
                        x: r.x,
                        y: r.y,
                        width: cw,
                        height: ch,
                    },
                    o.day_radius,
                ),
                vertical,
            );
            let formatted = value.map(|v| o.value_format.format(v)).transpose()?;
            let label = format!("{id}: {}", formatted.as_deref().unwrap_or(&o.empty_label));
            let owner = format!("year:{}", date.0);
            let visible = !o.hidden_years.contains(&date.0);
            if visible {
                let mut mark = Mark::new(id.clone(), geometry.clone(), color);
                mark.series = owner.clone();
                mark.label = label.clone();
                mark.value = value.unwrap_or(0.);
                mark.stroke = o.day_border;
                mark.opacity = o.day_opacity;
                scene.marks.push(mark);
                let label_value = match o.day_labels {
                    CalendarDayLabel::None => None,
                    CalendarDayLabel::Value => {
                        Some(formatted.clone().unwrap_or_else(|| o.empty_label.clone()))
                    }
                    CalendarDayLabel::DayOfMonth => Some(date.2.to_string()),
                    CalendarDayLabel::IsoDate => Some(id.clone()),
                };
                if let Some(value) = label_value {
                    let mut p = point([r.x + cw / 2., r.y + ch / 2.], vertical);
                    p[0] += o.day_label_offset[0];
                    p[1] += o.day_label_offset[1];
                    text(
                        &mut scene,
                        c,
                        p,
                        value,
                        TextAnchor::Middle,
                        &o.day_label_style,
                        Some(owner),
                    );
                }
            }
            month_cells
                .entry((date.0, date.1))
                .or_default()
                .push((week, wd, id.clone()));
            days.push(ComputedCalendarDay {
                date,
                id,
                value,
                provided,
                formatted_value: formatted,
                label,
                rect: rect(r, vertical),
                geometry,
                week,
                weekday: wd,
                year: date.0,
                month: date.1,
                color,
                visible,
            });
        }
        for ((year, month), cells) in month_cells {
            let month_offset = f64::from(month - 1) * o.month_spacing;
            let path = month_outline(
                &cells,
                left + o.day_spacing / 2. + month_offset,
                top + o.day_spacing / 2.,
                cw + o.day_spacing,
                ch + o.day_spacing,
                o.month_outline_padding,
            )?;
            let min_x = cells.iter().map(|(w, _, _)| *w).min().unwrap() as f64;
            let max_x = cells.iter().map(|(w, _, _)| *w).max().unwrap() as f64;
            let min_y = cells.iter().map(|(_, d, _)| *d).min().unwrap() as f64;
            let max_y = cells.iter().map(|(_, d, _)| *d).max().unwrap() as f64;
            let bounds = ClipRect {
                x: left + o.day_spacing / 2. + min_x * (cw + o.day_spacing)
                    - o.month_outline_padding
                    + month_offset,
                y: top + o.day_spacing / 2. + min_y * (ch + o.day_spacing)
                    - o.month_outline_padding,
                width: (max_x - min_x + 1.) * (cw + o.day_spacing) + 2. * o.month_outline_padding,
                height: (max_y - min_y + 1.) * (ch + o.day_spacing) + 2. * o.month_outline_padding,
            };
            let outline = transpose_geometry(path, vertical);
            let mark_id = if let Some(stroke) = o.month_border {
                let id = format!("calendar-month:{year:04}-{month:02}");
                let mut mark = Mark::new(id.clone(), outline.clone(), stroke.color);
                mark.fill = None;
                mark.stroke = Some(stroke);
                mark.label.clear();
                scene.marks.push(mark);
                Some(id)
            } else {
                None
            };
            if o.month_labels {
                let label = o
                    .month_names
                    .get(&(year, month))
                    .cloned()
                    .unwrap_or_else(|| o.months[usize::from(month - 1)].clone());
                let p = point(
                    [
                        bounds.x + bounds.width / 2.,
                        if before {
                            top - o.month_label_offset
                        } else {
                            top + h + o.month_label_offset
                        },
                    ],
                    vertical,
                );
                text(
                    &mut scene,
                    c,
                    p,
                    label,
                    if vertical {
                        if before {
                            TextAnchor::End
                        } else {
                            TextAnchor::Start
                        }
                    } else {
                        TextAnchor::Middle
                    },
                    &o.month_style,
                    None,
                );
            }
            months.push(ComputedCalendarMonth {
                year,
                month,
                day_ids: cells.into_iter().map(|(_, _, id)| id).collect(),
                outline,
                bounds: rect(bounds, vertical),
                mark_id,
            });
        }
        for &wd in &o.weekday_ticks {
            let p = point(
                [
                    left - o.weekday_offset,
                    top + o.day_spacing + f64::from(wd) * (ch + o.day_spacing) + ch / 2.,
                ],
                vertical,
            );
            text(
                &mut scene,
                c,
                p,
                o.weekdays[(usize::from(wd) + o.first_weekday.index()) % 7].clone(),
                if vertical {
                    TextAnchor::Middle
                } else {
                    TextAnchor::End
                },
                &o.weekday_style,
                None,
            );
        }
        if show_years {
            let label = year.map_or_else(
                || {
                    if first_year == last_year {
                        first_year.to_string()
                    } else {
                        format!("{first_year}–{last_year}")
                    }
                },
                |y| y.to_string(),
            );
            let p = point(
                [
                    canonical.margin.left + o.weekday_space,
                    canonical.margin.top
                        + index as f64 * (slot + o.year_gap)
                        + o.year_style.size(c) / 2.,
                ],
                vertical,
            );
            text(
                &mut scene,
                c,
                p,
                label,
                TextAnchor::Start,
                &o.year_style,
                None,
            );
        }
    }
    if o.year_legend {
        for year in first_year..=last_year {
            scene.legend.push(LegendItem {
                id: format!("year:{year}"),
                label: year.to_string(),
                color: c.theme.text,
            });
        }
    }
    scene.validate()?;
    Ok(CalendarLayout {
        scene,
        days,
        months,
        panels,
        start: civil(start),
        end: civil(end),
        color_scale: scale,
    })
}

/// Genuine local-calendar aggregation; no system zone, implicit I/O, or
/// assumption that a civil day is always 24 hours. Explicit null source cells
/// are available in `calendar_layout`; instant observations are finite values.
pub fn calendar_layout_zoned(
    c: &ChartConfig,
    data: &[CalendarInstant],
    zone: &TimeZone,
    aggregation: CalendarAggregation,
    o: &CalendarLayoutOptions,
) -> Result<CalendarLayout> {
    calendar_layout_zoned_with_color_profile(
        c,
        data,
        zone,
        aggregation,
        o,
        crate::ColorInterpolation::Srgb,
    )
}

/// Aggregate by local day and use the selected color interpolation for the result.
pub fn calendar_layout_zoned_with_color_profile(
    c: &ChartConfig,
    data: &[CalendarInstant],
    zone: &TimeZone,
    aggregation: CalendarAggregation,
    o: &CalendarLayoutOptions,
    profile: impl Into<crate::ColorScaleProfile>,
) -> Result<CalendarLayout> {
    ensure(
        data.len() <= MAX_CALENDAR_INSTANTS,
        "calendar instant budget exceeded",
    )?;
    let local = data
        .iter()
        .map(|d| calendar_date_at(d.timestamp_ms, d.value, zone).map(CalendarCell::from))
        .collect::<Result<Vec<_>>>()?;
    let (start, end) = range(&local, o)?;
    let dates = aggregate_calendar_instants(data, zone, civil(start), civil(end), aggregation)?;
    calendar_layout_with_color_profile(
        c,
        &dates
            .into_iter()
            .map(CalendarCell::from)
            .collect::<Vec<_>>(),
        &CalendarLayoutOptions {
            start: Some(civil(start)),
            end: Some(civil(end)),
            ..o.clone()
        },
        profile,
    )
}

fn month_outline(
    cells: &[(usize, usize, String)],
    left: f64,
    top: f64,
    pitch_x: f64,
    pitch_y: f64,
    padding: f64,
) -> Result<Geometry> {
    type Vertex = (usize, usize);
    // At most 31 rectangles per civil month. Coordinate compression computes
    // their exact orthogonal union, including padded diagonal contacts, in a
    // bounded grid (at most 61*13 elementary rectangles for calendar rows).
    let rectangles: Vec<_> = cells
        .iter()
        .map(|(x, y, _)| {
            [
                left + *x as f64 * pitch_x - padding,
                top + *y as f64 * pitch_y - padding,
                left + (x + 1) as f64 * pitch_x + padding,
                top + (y + 1) as f64 * pitch_y + padding,
            ]
        })
        .collect();
    let mut xs: Vec<_> = rectangles.iter().flat_map(|r| [r[0], r[2]]).collect();
    let mut ys: Vec<_> = rectangles.iter().flat_map(|r| [r[1], r[3]]).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    let mut edges = BTreeSet::<(Vertex, Vertex)>::new();
    for x in 0..xs.len() - 1 {
        for y in 0..ys.len() - 1 {
            if !rectangles
                .iter()
                .any(|r| r[0] <= xs[x] && r[2] >= xs[x + 1] && r[1] <= ys[y] && r[3] >= ys[y + 1])
            {
                continue;
            }
            let v = [(x, y), (x + 1, y), (x + 1, y + 1), (x, y + 1)];
            for i in 0..4 {
                let edge = (v[i], v[(i + 1) % 4]);
                if !edges.remove(&(edge.1, edge.0)) {
                    edges.insert(edge);
                }
            }
        }
    }
    let mut next = BTreeMap::<Vertex, BTreeSet<Vertex>>::new();
    for (a, b) in edges {
        next.entry(a).or_default().insert(b);
    }
    let mut commands = vec![];
    while let Some((&start, _)) = next.first_key_value() {
        let mut vertices = vec![start];
        let mut cursor = start;
        let mut previous = None;
        loop {
            let outgoing = next
                .get_mut(&cursor)
                .ok_or_else(|| ChartError("calendar month boundary is not closed".into()))?;
            let direction = |a: Vertex, b: Vertex| {
                if b.0 > a.0 {
                    0
                } else if b.1 > a.1 {
                    1
                } else if b.0 < a.0 {
                    2
                } else {
                    3
                }
            };
            let end = *outgoing
                .iter()
                .min_by_key(|candidate| {
                    previous.map_or(0, |prev| {
                        [1, 0, 3, 2]
                            [(direction(cursor, **candidate) + 4 - direction(prev, cursor)) % 4]
                    })
                })
                .unwrap();
            outgoing.remove(&end);
            if outgoing.is_empty() {
                next.remove(&cursor);
            }
            if end == start {
                break;
            }
            vertices.push(end);
            previous = Some(cursor);
            cursor = end;
            ensure(
                vertices.len() <= MAX_CALENDAR_LAYOUT_DAYS * 4,
                "calendar boundary exceeds budget",
            )?;
        }
        let corners: Vec<_> = vertices
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                let a = vertices[(i + vertices.len() - 1) % vertices.len()];
                let b = vertices[(i + 1) % vertices.len()];
                !(a.0 == p.0 && p.0 == b.0 || a.1 == p.1 && p.1 == b.1)
            })
            .map(|(_, p)| *p)
            .collect();
        for (i, v) in corners.iter().enumerate() {
            let p = P::new(xs[v.0], ys[v.1]);
            commands.push(if i == 0 { C::Move(p) } else { C::Line(p) });
        }
        commands.push(C::Close);
    }
    Ok(Geometry::Path(commands))
}
