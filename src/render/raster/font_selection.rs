//! Explicit aliases for statically instantiated, application-supplied faces.
use glory::style::Family;

use super::*;
use crate::{FontOptions, ResolvedFont};

/// Bind an application font-family alias and exact CSS weight to supplied bytes.
/// Family aliases need not equal the font's embedded name. The weight must
/// equal the face's actual OS/2 weight; variable fonts and synthetic weights
/// require an explicit future shaping profile rather than implicit emulation.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceDescriptor {
    pub family: Family,
    pub weight: f64,
}

fn same_family(a: &Family, b: &Family) -> bool {
    match (a, b) {
        (Family::Named(a), Family::Named(b)) => a.eq_ignore_ascii_case(b),
        _ => a == b,
    }
}

// CSS Fonts 4 §5: the 400..=500 search is intentionally not nearest distance.
// Rank static faces only; no synthetic bolding or variable axis is introduced.
fn weight_rank(requested: f64, available: f64) -> (u8, f64) {
    if (400. ..=500.).contains(&requested) {
        if (requested..=500.).contains(&available) {
            (0, available - requested)
        } else if available < requested {
            (1, requested - available)
        } else {
            (2, available - 500.)
        }
    } else if requested < 400. {
        if available <= requested {
            (0, requested - available)
        } else {
            (1, available - requested)
        }
    } else if available >= requested {
        (0, available - requested)
    } else {
        (1, requested - available)
    }
}

impl FontRegistry {
    /// Add a deterministic family/weight alias. Existing bindings, registered
    /// bytes and fallback order remain unchanged on rejection. Repeating the
    /// same binding is allowed; a different face cannot replace it implicitly.
    pub fn bind_face(&mut self, id: FontId, descriptor: FaceDescriptor) -> Result<()> {
        FontOptions {
            family: Some(vec![descriptor.family.clone()]),
            weight: Some(descriptor.weight),
        }
        .validate()?;
        let font = self
            .fonts
            .get(id.0)
            .ok_or_else(|| ChartError("font binding refers to an unregistered face".into()))?;
        let face = ttf_parser::Face::parse(&font.bytes, font.index).unwrap();
        ensure(
            face.tables().os2.is_some(),
            "font family bindings require an explicit OS/2 weight",
        )?;
        ensure(
            !face.is_variable(),
            "font family bindings require a static font face",
        )?;
        ensure(
            f64::from(face.weight().to_number()) == descriptor.weight,
            "font binding weight differs from the supplied face's actual OS/2 weight",
        )?;
        if let Some((bound_id, _)) = self.bindings.iter().find(|(_, existing)| {
            same_family(&existing.family, &descriptor.family)
                && existing.weight == descriptor.weight
        }) {
            return ensure(
                *bound_id == id,
                "font family/weight is already bound to another face",
            );
        }
        ensure(
            self.bindings.len() < 256,
            "font binding count budget exceeded",
        )?;
        self.bindings.push((id, descriptor));
        Ok(())
    }

    /// Select static faces in authored family order using CSS weight matching.
    /// Unbound families are skipped; at least one explicit binding is required.
    /// Generic families are application aliases, without system font discovery.
    /// Coverage fallback then uses the existing horizontal, bidi or vertical
    /// shaping profile. Font bytes are shared through Arc, never recopied.
    pub(super) fn for_font(&self, font: &ResolvedFont) -> Result<Self> {
        FontOptions {
            family: Some(font.family.clone()),
            weight: Some(font.weight),
        }
        .validate()?;
        let mut selected = self.clone();
        selected.fonts.clear();
        selected.bindings.clear();
        selected.bytes = 0;
        for family in &font.family {
            let Some((id, _)) = self
                .bindings
                .iter()
                .filter(|(_, descriptor)| same_family(&descriptor.family, family))
                .min_by(|(_, a), (_, b)| {
                    let a = weight_rank(font.weight, a.weight);
                    let b = weight_rank(font.weight, b.weight);
                    a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))
                })
            else {
                continue;
            };
            // Repeated aliases of one face do not multiply fallback work.
            let value = &self.fonts[id.0];
            if !selected.fonts.iter().any(|existing| {
                Arc::ptr_eq(&existing.bytes, &value.bytes) && existing.index == value.index
            }) {
                selected.bytes += value.bytes.len();
                selected.fonts.push(value.clone());
            }
        }
        ensure(
            !selected.fonts.is_empty(),
            "requested font families have no registered face binding",
        )?;
        Ok(selected)
    }

    /// Font-aware counterpart to `measure`, with shared bound-face selection.
    pub fn measure_with_font(
        &self,
        text: &str,
        size: f64,
        font: &ResolvedFont,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextMetrics> {
        self.for_font(font)?.measure(text, size, missing)
    }

    pub fn measure_layout_with_font(
        &self,
        text: &str,
        size: f64,
        font: &ResolvedFont,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextLayoutMetrics> {
        self.for_font(font)?.measure_layout(text, size, missing)
    }

    pub fn measure_label_with_font(
        &self,
        label: &Label,
        font: &ResolvedFont,
        missing: MissingGlyphPolicy,
    ) -> Result<Option<RasterTextBounds>> {
        self.for_font(font)?.measure_label(label, missing)
    }

    pub fn outline_label_with_font(
        &self,
        label: &Label,
        font: &ResolvedFont,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextOutline> {
        self.for_font(font)?.outline_label(label, missing)
    }

    pub fn painted_label_with_font(
        &self,
        label: &Label,
        font: &ResolvedFont,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterPaintedText> {
        self.for_font(font)?.painted_label(label, missing)
    }
}

#[cfg(test)]
mod tests {
    use super::weight_rank;
    #[test]
    fn css_weight_search_order_includes_the_non_nearest_400_to_500_exception() {
        for (request, expected) in [
            (350., [300, 100, 400, 500, 600, 700, 900]),
            (400., [400, 500, 300, 100, 600, 700, 900]),
            (450., [500, 400, 300, 100, 600, 700, 900]),
            (500., [500, 400, 300, 100, 600, 700, 900]),
            (550., [600, 700, 900, 500, 400, 300, 100]),
            (800., [900, 700, 600, 500, 400, 300, 100]),
        ] {
            let mut actual = [900, 100, 700, 300, 600, 400, 500];
            actual.sort_by(|a, b| {
                let a = weight_rank(request, f64::from(*a));
                let b = weight_rank(request, f64::from(*b));
                a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))
            });
            assert_eq!(actual, expected, "requested {request}");
        }
    }
}
