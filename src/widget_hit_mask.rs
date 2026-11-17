//! Exact alpha-positive pixel coverage for native browser hit companions.
//! RGB is irrelevant; alpha 1 is as eligible as alpha 255. Rectangles merge
//! vertically only when adjacent rows have the identical horizontal interval.
use crate::scene::ensure;
use crate::{ChartError, Result};

const MAX_PIXELS: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AlphaRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Partition exactly the pixels whose alpha is positive. The rectangle budget
/// counts finished and currently active rectangles together. `max_rects == 0`
/// accepts only a wholly transparent bitmap. Images require positive dimensions,
/// an exact RGBA byte length, and at most 67,108,864 pixels. Hosts separately
/// account for cumulative live/pending backing stores and readback memory.
pub(crate) fn alpha_rectangles(
    rgba: &[u8],
    width: u32,
    height: u32,
    max_rects: usize,
) -> Result<Vec<AlphaRect>> {
    collect(rgba, width, height, max_rects, &mut 0)
}

fn charge(work: &mut usize, limit: usize) -> Result<()> {
    let next = work
        .checked_add(1)
        .ok_or_else(|| ChartError("alpha hit-mask work overflow".into()))?;
    ensure(next <= limit, "alpha hit-mask work budget exceeded")?;
    *work = next;
    Ok(())
}

fn reserve(created: &mut usize, max_rects: usize, work: &mut usize, limit: usize) -> Result<()> {
    charge(work, limit)?;
    let next = created
        .checked_add(1)
        .ok_or_else(|| ChartError("alpha hit-mask rectangle count overflow".into()))?;
    ensure(
        next <= max_rects,
        "alpha hit-mask rectangle budget exceeded",
    )?;
    *created = next;
    Ok(())
}

// The extra work output is private instrumentation. Public and unit-test calls
// execute the same scanner, including every checked work charge and early exit.
fn collect(
    rgba: &[u8],
    width: u32,
    height: u32,
    max_rects: usize,
    work: &mut usize,
) -> Result<Vec<AlphaRect>> {
    *work = 0;
    ensure(
        width > 0 && height > 0,
        "alpha hit-mask dimensions must be positive",
    )?;
    let w =
        usize::try_from(width).map_err(|_| ChartError("alpha hit-mask width overflow".into()))?;
    let h =
        usize::try_from(height).map_err(|_| ChartError("alpha hit-mask height overflow".into()))?;
    let pixels = w
        .checked_mul(h)
        .ok_or_else(|| ChartError("alpha hit-mask pixel count overflow".into()))?;
    ensure(pixels <= MAX_PIXELS, "alpha hit-mask pixel budget exceeded")?;
    let bytes = pixels
        .checked_mul(4)
        .ok_or_else(|| ChartError("alpha hit-mask byte count overflow".into()))?;
    ensure(
        rgba.len() == bytes,
        "alpha hit-mask RGBA byte length mismatch",
    )?;
    let row_bytes = w
        .checked_mul(4)
        .ok_or_else(|| ChartError("alpha hit-mask row byte overflow".into()))?;
    let limit = pixels
        .checked_mul(8)
        .ok_or_else(|| ChartError("alpha hit-mask work limit overflow".into()))?;
    let mut completed = Vec::new();
    let mut active: Vec<AlphaRect> = Vec::new();
    let mut next = Vec::new();
    let mut created = 0usize;
    for (y, row) in rgba.chunks_exact(row_bytes).enumerate() {
        charge(work, limit)?; // Charge even a fully transparent row header.
        next.clear();
        let mut previous = 0usize;
        let mut x = 0usize;
        while x < w {
            charge(work, limit)?;
            // Checked row_bytes and x < w prove this pixel index in bounds.
            if row[x * 4 + 3] == 0 {
                x += 1;
                continue;
            }
            let start = x;
            while previous < active.len() && (active[previous].x as usize) < start {
                charge(work, limit)?;
                completed.push(active[previous]);
                previous += 1;
            }
            let same_start = previous < active.len() && active[previous].x as usize == start;
            let mut reserved = !same_start;
            if !same_start {
                // A different start cannot extend an active rectangle. Check
                // the new rectangle before scanning a potentially very long run.
                reserve(&mut created, max_rects, work, limit)?;
            }
            x += 1;
            while x < w {
                charge(work, limit)?;
                if row[x * 4 + 3] == 0 {
                    break;
                }
                if same_start && !reserved && x - start >= active[previous].width as usize {
                    // The current run has already outgrown its only possible
                    // predecessor; a new rectangle is certain before its tail.
                    reserve(&mut created, max_rects, work, limit)?;
                    reserved = true;
                }
                x += 1;
            }
            let run_width = u32::try_from(x - start)
                .map_err(|_| ChartError("alpha hit-mask run width overflow".into()))?;
            charge(work, limit)?;
            if same_start && active[previous].width == run_width {
                let mut rectangle = active[previous];
                rectangle.height = rectangle
                    .height
                    .checked_add(1)
                    .ok_or_else(|| ChartError("alpha hit-mask rectangle height overflow".into()))?;
                next.push(rectangle);
                previous += 1;
            } else {
                if same_start {
                    if !reserved {
                        reserve(&mut created, max_rects, work, limit)?;
                    }
                    completed.push(active[previous]);
                    previous += 1;
                }
                next.push(AlphaRect {
                    x: u32::try_from(start)
                        .map_err(|_| ChartError("alpha hit-mask x overflow".into()))?,
                    y: u32::try_from(y)
                        .map_err(|_| ChartError("alpha hit-mask y overflow".into()))?,
                    width: run_width,
                    height: 1,
                });
            }
            // The run's terminating transparent pixel was already examined.
            if x < w {
                x += 1;
            }
        }
        for rectangle in &active[previous..] {
            charge(work, limit)?;
            completed.push(*rectangle);
        }
        std::mem::swap(&mut active, &mut next);
    }
    for rectangle in active {
        charge(work, limit)?;
        completed.push(rectangle);
    }
    debug_assert_eq!(completed.len(), created);
    Ok(completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bitmap(alpha: &[u8]) -> Vec<u8> {
        alpha.iter().flat_map(|a| [220, 40, 90, *a]).collect()
    }
    fn check_union(rgba: &[u8], w: u32, h: u32, rectangles: &[AlphaRect]) {
        let mut counts = vec![0u8; (w * h) as usize];
        for r in rectangles {
            assert!(r.width > 0 && r.height > 0);
            assert!(
                r.x.checked_add(r.width).unwrap() <= w && r.y.checked_add(r.height).unwrap() <= h
            );
            for y in r.y..r.y + r.height {
                for x in r.x..r.x + r.width {
                    counts[(y * w + x) as usize] += 1;
                }
            }
        }
        for (i, count) in counts.into_iter().enumerate() {
            assert_eq!(
                count,
                u8::from(rgba[i * 4 + 3] > 0),
                "pixel {i}: exact, disjoint coverage"
            );
        }
    }

    #[test]
    fn every_positive_alpha_is_preserved_and_transparent_rgb_is_ignored() {
        let rgba = [255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 255];
        let result = alpha_rectangles(&rgba, 4, 1, 1).unwrap();
        assert_eq!(
            result,
            [AlphaRect {
                x: 1,
                y: 0,
                width: 3,
                height: 1
            }]
        );
        check_union(&rgba, 4, 1, &result);
        assert!(alpha_rectangles(&rgba, 4, 1, 0).is_err());
        assert!(
            alpha_rectangles(&bitmap(&[0; 12]), 3, 4, 0)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            alpha_rectangles(&bitmap(&[1]), 1, 1, usize::MAX)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn only_identical_adjacent_intervals_merge_and_holes_remain_empty() {
        let rgba = bitmap(&[1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1]);
        let result = alpha_rectangles(&rgba, 4, 3, 2).unwrap();
        assert_eq!(
            result,
            [
                AlphaRect {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 3
                },
                AlphaRect {
                    x: 3,
                    y: 0,
                    width: 1,
                    height: 3
                }
            ]
        );
        let hole = bitmap(&[1, 1, 1, 1, 0, 1, 1, 1, 1]);
        let result = alpha_rectangles(&hole, 3, 3, 4).unwrap();
        assert_eq!(result.len(), 4);
        check_union(&hole, 3, 3, &result);
        assert!(alpha_rectangles(&hole, 3, 3, 3).is_err());
        let changing = bitmap(&[0, 1, 1, 0, 0, 1, 1, 1]);
        let result = alpha_rectangles(&changing, 4, 2, 2).unwrap();
        assert_eq!(result.len(), 2);
        check_union(&changing, 4, 2, &result);
    }

    #[test]
    fn completed_and_active_rectangles_share_one_exact_budget() {
        let rgba = bitmap(&[1, 0, 0, 0, 0, 1]);
        assert!(alpha_rectangles(&rgba, 3, 2, 1).is_err());
        let result = alpha_rectangles(&rgba, 3, 2, 2).unwrap();
        assert_eq!(result.len(), 2);
        check_union(&rgba, 3, 2, &result);
        let dashed = bitmap(&[1, 0, 1, 0, 1, 0, 1, 0, 1]);
        assert_eq!(alpha_rectangles(&dashed, 9, 1, 5).unwrap().len(), 5);
        assert!(alpha_rectangles(&dashed, 9, 1, 4).is_err());
    }

    #[test]
    fn deterministic_random_bitmaps_have_exact_union_and_exact_budget() {
        let mut seed = 0x215af032_u64;
        let mut random = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed
        };
        for _ in 0..1000 {
            let w = (random() % 19 + 1) as u32;
            let h = (random() % 17 + 1) as u32;
            let rgba = (0..w * h)
                .flat_map(|_| {
                    let bits = random();
                    let a = match bits % 5 {
                        0 | 1 => 0,
                        2 => 1,
                        3 => 128,
                        _ => 255,
                    };
                    [bits as u8, (bits >> 8) as u8, (bits >> 16) as u8, a]
                })
                .collect::<Vec<_>>();
            let before = rgba.clone();
            let mut work = 0;
            let result = collect(&rgba, w, h, usize::MAX, &mut work).unwrap();
            assert!(work <= (w * h * 8) as usize);
            check_union(&rgba, w, h, &result);
            assert_eq!(alpha_rectangles(&rgba, w, h, result.len()).unwrap(), result);
            if !result.is_empty() {
                assert!(alpha_rectangles(&rgba, w, h, result.len() - 1).is_err());
            }
            assert_eq!(rgba, before);
        }
    }

    #[test]
    fn dimensions_length_pixel_cap_and_work_arithmetic_are_checked_before_scanning() {
        for (w, h) in [(0, 1), (1, 0), (0, 0), (u32::MAX, u32::MAX), (u32::MAX, 1)] {
            let mut work = 99;
            assert!(collect(&[], w, h, 1, &mut work).is_err());
            assert_eq!(work, 0);
        }
        let mut work = 99;
        assert!(
            collect(&[], MAX_PIXELS as u32 + 1, 1, 1, &mut work)
                .unwrap_err()
                .0
                .contains("pixel budget")
        );
        assert_eq!(work, 0);
        assert!(
            collect(&[], MAX_PIXELS as u32, 1, 1, &mut work)
                .unwrap_err()
                .0
                .contains("byte length")
        );
        assert_eq!(work, 0);
        for bytes in [vec![], vec![0; 3], vec![0; 5]] {
            assert!(alpha_rectangles(&bytes, 1, 1, 0).is_err());
        }
        let mut overflow = usize::MAX;
        assert!(charge(&mut overflow, usize::MAX).is_err());
        assert_eq!(overflow, usize::MAX);
    }

    #[test]
    fn adversarial_long_rows_reject_at_the_first_unaffordable_new_interval() {
        let width = 1_000_000;
        let mut rgba = vec![255; width * 4];
        let mut work = 0;
        assert!(collect(&rgba, width as u32, 1, 0, &mut work).is_err());
        assert!(
            work <= 8,
            "zero budget must not scan a million-pixel opaque run: {work}"
        );
        rgba[7] = 0;
        assert!(collect(&rgba, width as u32, 1, 1, &mut work).is_err());
        assert!(
            work <= 16,
            "second new interval must fail before its long tail: {work}"
        );
        let mut growing = vec![0; width * 8];
        growing[3] = 1;
        growing[width * 4..].fill(255);
        assert!(collect(&growing, width as u32, 2, 1, &mut work).is_err());
        assert!(
            work <= width + 16,
            "outgrown predecessor must reject before the second row's tail: {work}"
        );
        let tall = bitmap(&vec![1; 10_000]);
        let result = alpha_rectangles(&tall, 1, 10_000, 1).unwrap();
        assert_eq!(
            result,
            [AlphaRect {
                x: 0,
                y: 0,
                width: 1,
                height: 10_000
            }]
        );
    }
}
