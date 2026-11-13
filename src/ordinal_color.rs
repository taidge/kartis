//! Typed ordinal color scales with explicit range, domain and unknown handling.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;
use std::str::FromStr;

use crate::scene::{MAX_MARKS, ensure};
use crate::{CategoryKey, ChartError, Color, ColorScheme, Result};

const MAX_ORDINAL_COLORS: usize = 256;
const MAX_DOMAIN_TEXT_BYTES: usize = 4 * 1024 * 1024;

const fn rgb(hex: u32) -> Color {
    Color((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

// Nivo's categoricalColorSchemes and d3-scale-chromatic 3.1.0.
// The Nivo MIT and D3 ISC/ColorBrewer notices are in the third-party notices
// and src/color_scale/schemes/LICENSE-d3-scale-chromatic.txt, respectively.
macro_rules! categorical_schemes {
    ($($variant:ident => ($id:literal, [$($color:literal),+ $(,)?])),+ $(,)?) => {
        /// A categorical palette from Nivo's color scheme catalog.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum CategoricalColorScheme {
            $($variant),+
        }

        impl CategoricalColorScheme {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }

            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($id => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// The entire official palette in authored order.
            pub const fn palette(self) -> &'static [Color] {
                match self {
                    $(Self::$variant => {
                        const COLORS: &[Color] = &[$(rgb($color)),+];
                        COLORS
                    }),+
                }
            }
        }
    };
}

categorical_schemes! {
    Nivo => ("nivo", [
        0xe8c1a0, 0xf47560, 0xf1e15b, 0xe8a838, 0x61cdbb, 0x97e3d5,
    ]),
    Category10 => ("category10", [
        0x1f77b4, 0xff7f0e, 0x2ca02c, 0xd62728, 0x9467bd,
        0x8c564b, 0xe377c2, 0x7f7f7f, 0xbcbd22, 0x17becf,
    ]),
    Accent => ("accent", [
        0x7fc97f, 0xbeaed4, 0xfdc086, 0xffff99, 0x386cb0, 0xf0027f, 0xbf5b17, 0x666666,
    ]),
    Dark2 => ("dark2", [
        0x1b9e77, 0xd95f02, 0x7570b3, 0xe7298a, 0x66a61e, 0xe6ab02, 0xa6761d, 0x666666,
    ]),
    Paired => ("paired", [
        0xa6cee3, 0x1f78b4, 0xb2df8a, 0x33a02c, 0xfb9a99, 0xe31a1c,
        0xfdbf6f, 0xff7f00, 0xcab2d6, 0x6a3d9a, 0xffff99, 0xb15928,
    ]),
    Pastel1 => ("pastel1", [
        0xfbb4ae, 0xb3cde3, 0xccebc5, 0xdecbe4, 0xfed9a6,
        0xffffcc, 0xe5d8bd, 0xfddaec, 0xf2f2f2,
    ]),
    Pastel2 => ("pastel2", [
        0xb3e2cd, 0xfdcdac, 0xcbd5e8, 0xf4cae4, 0xe6f5c9, 0xfff2ae, 0xf1e2cc, 0xcccccc,
    ]),
    Set1 => ("set1", [
        0xe41a1c, 0x377eb8, 0x4daf4a, 0x984ea3, 0xff7f00,
        0xffff33, 0xa65628, 0xf781bf, 0x999999,
    ]),
    Set2 => ("set2", [
        0x66c2a5, 0xfc8d62, 0x8da0cb, 0xe78ac3, 0xa6d854, 0xffd92f, 0xe5c494, 0xb3b3b3,
    ]),
    Set3 => ("set3", [
        0x8dd3c7, 0xffffb3, 0xbebada, 0xfb8072, 0x80b1d3, 0xfdb462,
        0xb3de69, 0xfccde5, 0xd9d9d9, 0xbc80bd, 0xccebc5, 0xffed6f,
    ]),
    Tableau10 => ("tableau10", [
        0x4e79a7, 0xf28e2c, 0xe15759, 0x76b7b2, 0x59a14f,
        0xedc949, 0xaf7aa1, 0xff9da7, 0x9c755f, 0xbab0ab,
    ]),
}

impl fmt::Display for CategoricalColorScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for CategoricalColorScheme {
    type Err = ChartError;

    fn from_str(value: &str) -> Result<Self> {
        Self::from_name(value).ok_or_else(|| ChartError("unknown categorical color scheme".into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrdinalColorPalette {
    Custom(Vec<Color>),
    Categorical(CategoricalColorScheme),
    /// Use the exact ColorBrewer palette for this size. Without a size, use
    /// the largest official palette, as Nivo does for ordinal scales.
    Brewer {
        scheme: ColorScheme,
        size: Option<usize>,
    },
}

impl OrdinalColorPalette {
    fn resolve(&self) -> Result<Vec<Color>> {
        match self {
            Self::Custom(colors) => {
                ensure(
                    (1..=MAX_ORDINAL_COLORS).contains(&colors.len()),
                    "ordinal color palette needs 1 to 256 colors",
                )?;
                Ok(colors.clone())
            }
            Self::Categorical(scheme) => Ok(scheme.palette().to_vec()),
            Self::Brewer { scheme, size } => {
                let sizes = scheme.palette_sizes().ok_or_else(|| {
                    ChartError("ordinal ColorBrewer palette needs a discrete scheme".into())
                })?;
                scheme.palette(size.unwrap_or(*sizes.end()))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrdinalColorUnknown {
    #[default]
    Missing,
    Color(Color),
    /// Append unseen keys only when explicitly calling
    /// `OrdinalColorScale::map_or_insert`.
    Extend,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OrdinalColorOverflow {
    /// Reuse palette colors in order when the domain exceeds the range.
    #[default]
    Cycle,
    /// Apply the unknown-color fallback after the last palette color.
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrdinalColorOptions {
    pub palette: OrdinalColorPalette,
    /// A supplied domain fixes the initial key order and membership.
    /// Otherwise the scale uses all source keys in first-occurrence order.
    /// Include hidden source keys to keep their palette positions reserved.
    pub domain: Option<Vec<CategoryKey>>,
    pub unknown: OrdinalColorUnknown,
    pub overflow: OrdinalColorOverflow,
}

impl Default for OrdinalColorOptions {
    fn default() -> Self {
        Self {
            palette: OrdinalColorPalette::Categorical(CategoricalColorScheme::Nivo),
            domain: None,
            unknown: OrdinalColorUnknown::Missing,
            overflow: OrdinalColorOverflow::Cycle,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrdinalColorScale {
    domain: Vec<CategoryKey>,
    indices: BTreeMap<String, usize>,
    palette: Vec<Color>,
    unknown: OrdinalColorUnknown,
    overflow: OrdinalColorOverflow,
    text_bytes: usize,
}

impl OrdinalColorOptions {
    pub fn resolve(
        &self,
        input: impl IntoIterator<Item = CategoryKey>,
    ) -> Result<OrdinalColorScale> {
        let mut scale = OrdinalColorScale {
            domain: Vec::new(),
            indices: BTreeMap::new(),
            palette: self.palette.resolve()?,
            unknown: self.unknown,
            overflow: self.overflow,
            text_bytes: 0,
        };
        match &self.domain {
            Some(domain) => scale.seed(domain.iter().cloned())?,
            None => scale.seed(input)?,
        }
        Ok(scale)
    }
}

impl OrdinalColorScale {
    pub fn domain(&self) -> &[CategoryKey] {
        &self.domain
    }

    pub fn palette(&self) -> &[Color] {
        &self.palette
    }

    /// Return the color for a domain position, including its overflow policy.
    pub fn color_at(&self, index: usize) -> Option<Color> {
        if index >= self.domain.len() {
            return None;
        }
        match self.overflow {
            OrdinalColorOverflow::Cycle => Some(self.palette[index % self.palette.len()]),
            OrdinalColorOverflow::Unknown => {
                self.palette.get(index).copied().or_else(|| self.fallback())
            }
        }
    }

    /// A read-only lookup. Invalid keys return no color.
    pub fn map(&self, key: &CategoryKey) -> Option<Color> {
        let id = key.identity().ok()?;
        match self.indices.get(&id) {
            Some(&index) => self.color_at(index),
            None => self.fallback(),
        }
    }

    /// Assign a newly seen key when the unknown policy is `Extend`.
    /// Invalid keys and exhausted domain budgets leave the scale unchanged.
    pub fn map_or_insert(&mut self, key: CategoryKey) -> Result<Option<Color>> {
        let id = key.identity()?;
        if let Some(&index) = self.indices.get(&id) {
            return Ok(self.color_at(index));
        }
        if self.unknown != OrdinalColorUnknown::Extend {
            return Ok(self.fallback());
        }
        let index = self.insert(key, id)?;
        Ok(self.color_at(index))
    }

    fn fallback(&self) -> Option<Color> {
        match self.unknown {
            OrdinalColorUnknown::Color(color) => Some(color),
            OrdinalColorUnknown::Missing | OrdinalColorUnknown::Extend => None,
        }
    }

    fn seed(&mut self, input: impl IntoIterator<Item = CategoryKey>) -> Result<()> {
        for (index, key) in input.into_iter().enumerate() {
            ensure(index < MAX_MARKS, "ordinal color source exceeds key budget")?;
            let id = key.identity()?;
            self.insert(key, id)?;
        }
        Ok(())
    }

    fn insert(&mut self, mut key: CategoryKey, id: String) -> Result<usize> {
        let entry = match self.indices.entry(id) {
            Entry::Occupied(entry) => return Ok(*entry.get()),
            Entry::Vacant(entry) => entry,
        };
        let index = self.domain.len();
        ensure(index < MAX_MARKS, "ordinal color domain exceeds key budget")?;
        let text_bytes = self.text_bytes
            + match &key {
                CategoryKey::String(text) => text.len(),
                CategoryKey::Number(_) => 0,
            };
        ensure(
            text_bytes <= MAX_DOMAIN_TEXT_BYTES,
            "ordinal color domain text budget exceeded",
        )?;
        if let CategoryKey::Number(number) = &mut key
            && *number == 0.
        {
            *number = 0.;
        }
        entry.insert(index);
        self.domain.push(key);
        self.text_bytes = text_bytes;
        Ok(index)
    }
}
