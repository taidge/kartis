//! Transactional, data-space axis filters over a complete parallel layout.
use super::*;

/// Missing values on an actively filtered axis. Unfiltered axes do not reject missing values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallelFilterMissing {
    #[default]
    Exclude,
    Include,
}

/// An inclusive interval in raw source units; equal endpoints select one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParallelFilterRange {
    pub bounds: [f64; 2],
    pub missing: ParallelFilterMissing,
}
impl ParallelFilterRange {
    pub fn new(bounds: [f64; 2]) -> Self {
        Self {
            bounds,
            missing: ParallelFilterMissing::Exclude,
        }
    }
    fn validate(self) -> Result<()> {
        ensure(
            self.bounds.into_iter().all(f64::is_finite) && self.bounds[0] <= self.bounds[1],
            "invalid parallel filter interval",
        )
    }
}

/// Owns the full source, never a previously filtered subset. Every update is admitted
/// before replacing either the filters, source data, or drawable frame.
#[derive(Clone, Debug)]
pub struct ParallelFilterController {
    config: ChartConfig,
    dimensions: Vec<ParallelDimension>,
    rows: Vec<ParallelDatum>,
    options: ParallelCoordinatesOptions,
    profiles: BTreeMap<String, ParallelScaleOptions>,
    filters: BTreeMap<String, ParallelFilterRange>,
    layout: ParallelCoordinatesLayout,
}
impl ParallelFilterController {
    pub fn new(
        config: &ChartConfig,
        dimensions: &[ParallelDimension],
        rows: &[ParallelDatum],
        options: &ParallelCoordinatesOptions,
        profiles: &BTreeMap<String, ParallelScaleOptions>,
    ) -> Result<Self> {
        let layout =
            parallel_layout_with_scale_options(config, dimensions, rows, options, profiles)?;
        Ok(Self {
            config: config.clone(),
            dimensions: dimensions.to_vec(),
            rows: rows.to_vec(),
            options: options.clone(),
            profiles: profiles.clone(),
            filters: BTreeMap::new(),
            layout,
        })
    }
    pub fn layout(&self) -> &ParallelCoordinatesLayout {
        &self.layout
    }
    pub fn filters(&self) -> &BTreeMap<String, ParallelFilterRange> {
        &self.filters
    }
    /// Atomically replace the complete AND filter set. Unknown dimensions reject.
    pub fn replace_filters(
        &mut self,
        filters: BTreeMap<String, ParallelFilterRange>,
    ) -> Result<()> {
        let layout = self.candidate(&self.rows, &filters)?;
        self.filters = filters;
        self.layout = layout;
        Ok(())
    }
    pub fn set_filter(
        &mut self,
        dimension: &str,
        filter: Option<ParallelFilterRange>,
    ) -> Result<()> {
        ensure(
            self.dimensions.iter().any(|d| d.id == dimension),
            "unknown parallel filter dimension",
        )?;
        let mut filters = self.filters.clone();
        if let Some(filter) = filter {
            filters.insert(dimension.to_owned(), filter);
        } else {
            filters.remove(dimension);
        }
        self.replace_filters(filters)
    }
    pub fn clear(&mut self) -> Result<()> {
        self.replace_filters(BTreeMap::new())
    }
    /// Replace all records and re-infer domains from the complete new source.
    /// Existing data-space intervals persist; any invalid proposal keeps the old frame.
    pub fn replace_data(&mut self, rows: &[ParallelDatum]) -> Result<()> {
        let layout = self.candidate(rows, &self.filters)?;
        self.rows = rows.to_vec();
        self.layout = layout;
        Ok(())
    }
    /// Convert an axis-coordinate interval in Scene logical pixels. Reversed
    /// endpoints are normalized; coordinates beyond the actual axis reject.
    /// UTC boundaries keep fractional milliseconds, avoiding inverse truncation.
    pub fn set_pixel_filter(
        &mut self,
        dimension: &str,
        pixels: Option<[f64; 2]>,
        missing: ParallelFilterMissing,
    ) -> Result<()> {
        let Some(mut pixels) = pixels else {
            return self.set_filter(dimension, None);
        };
        let axis = self
            .layout
            .dimensions
            .iter()
            .find(|d| d.id == dimension)
            .ok_or_else(|| ChartError("unknown parallel filter dimension".into()))?;
        ensure(
            pixels.into_iter().all(f64::is_finite),
            "invalid parallel filter pixels",
        )?;
        pixels.sort_by(f64::total_cmp);
        let range = axis.scale.range();
        ensure(
            pixels[0] >= range[0].min(range[1]) && pixels[1] <= range[0].max(range[1]),
            "parallel filter pixels outside axis",
        )?;
        let mut bounds = if axis.scale.kind() == ScaleKind::Utc {
            let domain = axis.scale.domain();
            pixels.map(|p| {
                domain[0] + (p - range[0]) / (range[1] - range[0]) * (domain[1] - domain[0])
            })
        } else {
            [axis.scale.invert(pixels[0])?, axis.scale.invert(pixels[1])?]
        };
        bounds.sort_by(f64::total_cmp);
        // Preserve a source value on an exact rendered endpoint despite inverse
        // roundoff. Clamped outliers do not masquerade as domain endpoint values.
        let index = self
            .layout
            .dimensions
            .iter()
            .position(|d| d.id == dimension)
            .unwrap();
        let domain = axis.scale.domain();
        for value in self.rows.iter().filter_map(|r| r.values[index]) {
            if value >= domain[0].min(domain[1])
                && value <= domain[0].max(domain[1])
                && pixels.contains(&axis.scale.map(value))
            {
                bounds[0] = bounds[0].min(value);
                bounds[1] = bounds[1].max(value);
            }
        }
        self.set_filter(dimension, Some(ParallelFilterRange { bounds, missing }))
    }
    /// Use Y for vertical variable axes (Horizontal layout), X for horizontal
    /// variable axes. Hosts must convert client/host coordinates to Scene first.
    pub fn set_brush_filter(
        &mut self,
        dimension: &str,
        selection: Option<Brush>,
        missing: ParallelFilterMissing,
    ) -> Result<()> {
        let pixels = if let Some(brush) = selection {
            brush.validate()?;
            Some(match self.options.layout {
                ParallelLayout::Horizontal => [brush.y0, brush.y1],
                ParallelLayout::Vertical => [brush.x0, brush.x1],
            })
        } else {
            None
        };
        self.set_pixel_filter(dimension, pixels, missing)
    }
    fn candidate(
        &self,
        rows: &[ParallelDatum],
        filters: &BTreeMap<String, ParallelFilterRange>,
    ) -> Result<ParallelCoordinatesLayout> {
        for (id, range) in filters {
            ensure(
                self.dimensions.iter().any(|d| &d.id == id),
                "unknown parallel filter dimension",
            )?;
            range.validate()?;
        }
        // Validate every record, including records rejected by the proposed filters.
        let full = parallel_layout_with_scale_options(
            &self.config,
            &self.dimensions,
            rows,
            &self.options,
            &self.profiles,
        )?;
        if filters.is_empty() {
            return Ok(full);
        }
        let selected = full
            .rows
            .iter()
            .filter(|row| {
                filters.iter().all(|(id, range)| {
                    let index = self.dimensions.iter().position(|d| &d.id == id).unwrap();
                    match row.datum.values[index] {
                        Some(value) => value >= range.bounds[0] && value <= range.bounds[1],
                        None => range.missing == ParallelFilterMissing::Include,
                    }
                })
            })
            .map(|row| row.datum.id.clone())
            .collect();
        build_parallel_selected(
            &self.config,
            &self.dimensions,
            rows,
            &self.options,
            Some(&self.profiles),
            Some(&selected),
        )
    }
}
