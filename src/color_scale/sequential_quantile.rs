//! Empirical right-rank colors and their exact, bounded data-axis legends.
use super::{ColorScale, ColorScaleStop, color_fraction};
use crate::{Color, Result, paint::MAX_GRADIENT_STOPS, scene::ensure};

impl ColorScale {
    /// Complete sorted sample distribution for SequentialQuantile only.
    /// Equal values retain input order, including the spelling of signed zero.
    /// Clones share this immutable distribution; an empty distribution is Some([]).
    pub fn sample_domain(&self) -> Option<&[f64]> {
        self.samples.as_deref()
    }

    /// Original right-bisect empirical rank, independent of the legend window.
    /// Finite queries saturate at 0/1. Fewer than two observations have no rank.
    pub fn rank(&self, value: f64) -> Option<f64> {
        let samples = self.samples.as_deref()?;
        if samples.len() < 2 || !value.is_finite() {
            return None;
        }
        let index = samples.partition_point(|sample| *sample <= value).max(1);
        Some((index - 1) as f64 / (samples.len() - 1) as f64)
    }

    /// Colors at ordinal ranks i/(n-1), as D3's sequentialQuantile.range().
    /// Repeated samples can have different ordinal colors, even though map(x)
    /// always uses the rightmost rank for that value. Empty -> [], singleton -> unknown.
    pub fn rank_colors(&self) -> Option<Vec<Color>> {
        let samples = self.samples.as_deref()?;
        Some(match samples.len() {
            0 => Vec::new(),
            1 => vec![self.unknown],
            n => (0..n)
                .map(|i| self.color_at_parameter(i as f64 / (n - 1) as f64))
                .collect(),
        })
    }

    /// R7 distribution quantiles at 0..=segments, separately from rank colors.
    /// Valid segments are 1..=256. Empty and other kinds return None;
    /// a singleton has repeated finite quantiles despite its undefined rank.
    /// Original R7 overflow is rejected, never replaced by a safe interpolation.
    pub fn quantiles(&self, segments: usize) -> Result<Option<Vec<f64>>> {
        ensure(
            (1..=256).contains(&segments),
            "quantile segments must be 1..=256",
        )?;
        let Some(samples) = self
            .samples
            .as_deref()
            .filter(|samples| !samples.is_empty())
        else {
            return Ok(None);
        };
        let minimum = samples
            .iter()
            .copied()
            .reduce(|a, b| if b < a { b } else { a })
            .unwrap();
        let maximum = samples
            .iter()
            .copied()
            .reduce(|a, b| if b > a { b } else { a })
            .unwrap();
        (0..=segments)
            .map(|i| {
                let value = if i == 0 || samples.len() == 1 {
                    minimum
                } else if i == segments {
                    maximum
                } else {
                    let index = (samples.len() - 1) as f64 * (i as f64 / segments as f64);
                    let lower = index.floor() as usize;
                    let a = samples[lower];
                    let b = samples[lower + 1];
                    // Preserve original R7 operation order, signed-zero behavior and overflow.
                    a + (b - a) * (index - lower as f64)
                };
                ensure(
                    value.is_finite(),
                    "sequential quantile R7 result is not finite",
                )?;
                Ok(value)
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    }

    pub(super) fn sequential_quantile_stops(&self) -> Result<Vec<ColorScaleStop>> {
        let [a, b] = self.domain;
        let mut color = self.map(Some(a));
        let stop = |offset, value, color| ColorScaleStop {
            offset,
            value,
            color,
        };
        let samples = self
            .samples
            .as_deref()
            .expect("sequential quantile distribution");
        if a == b || samples.len() < 2 {
            return Ok(vec![stop(0., a, color), stop(1., b, color)]);
        }
        let mut stops = vec![stop(0., a, color)];
        let mut previous = 0.;
        let mut previous_f32 = 0_f32;
        let mut index = 0;
        while index < samples.len() {
            let value = samples[index];
            let end = index + samples[index..].partition_point(|sample| *sample == value);
            index = end;
            if value <= a {
                continue;
            }
            if value > b {
                break;
            }
            let next = self.color_at_parameter((end - 1) as f64 / (samples.len() - 1) as f64);
            // Merge only adjacent exactly equal RGBA plateaus; surviving boundaries never move.
            if next == color {
                continue;
            }
            let offset = color_fraction(value, a, b);
            let backend = offset as f32;
            ensure(
                offset.is_finite()
                    && offset > previous
                    && offset <= 1.
                    && backend > previous_f32
                    && backend <= 1.
                    && (value == b || offset < 1. && backend < 1.),
                "sequential quantile legend jumps collapse in f64 or backend f32",
            )?;
            ensure(
                stops.len() + 2 <= MAX_GRADIENT_STOPS,
                "sequential quantile legend exceeds 512 stops",
            )?;
            stops.push(stop(offset, value, color));
            stops.push(stop(offset, value, next));
            previous = offset;
            previous_f32 = backend;
            color = next;
        }
        if previous < 1. {
            ensure(
                stops.len() < MAX_GRADIENT_STOPS,
                "sequential quantile legend exceeds 512 stops",
            )?;
            stops.push(stop(1., b, color));
        }
        Ok(stops)
    }
}
