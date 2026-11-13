//! Repeating logical-coordinate pattern definitions inspired by Nivo's
//! `packages/core/src/components/defs/patterns/Pattern{Dots,Squares,Lines}.js`.
//! Spacing is center-to-center pitch, rather than Nivo's `size + padding`.
//! Staggered point patterns use the same two diagonal motifs in a 2*pitch tile.
//! Unlike angle-dependent rectangular stripe tiles, rotation transforms the
//! entire periodic lattice, including the background, without a 0/90° singularity.
use std::f64::consts::PI;
use std::fmt::Write;

use crate::scene::ensure;
use crate::{ChartError, Color, Geometry, Result};

pub const MAX_PATTERN_TILE_SIDE: u32 = 4096;
pub const MAX_PATTERN_TILE_PIXELS: u64 = 4_194_304;
pub const MAX_PATTERN_TILE_OPERATIONS: u64 = 67_108_864;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternShape {
    Dots {
        diameter: f64,
        stagger: bool,
    },
    Squares {
        size: f64,
        stagger: bool,
    },
    /// Horizontal stripes before rotation. Width zero draws only background.
    Lines {
        width: f64,
    },
}
impl Default for PatternShape {
    fn default() -> Self {
        Self::Dots {
            diameter: 4.,
            stagger: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PatternFill {
    pub shape: PatternShape,
    /// Center-to-center pitch in 0.25..=1024 logical Scene units. Motif size
    /// must be in 0..=spacing; foreground motifs never overlap themselves.
    pub spacing: f64,
    /// Clockwise degrees in the screen's Y-down coordinates, |angle|<=360000.
    pub rotation: f64,
    /// Logical translation after rotation; independent of a mark's bounds.
    pub offset: [f64; 2],
    pub foreground: Color,
    pub background: Color,
}
impl Default for PatternFill {
    fn default() -> Self {
        Self {
            shape: PatternShape::default(),
            spacing: 8.,
            rotation: 0.,
            offset: [0., 0.],
            foreground: Color(0, 0, 0, 255),
            background: Color(255, 255, 255, 255),
        }
    }
}
/// One periodic cell in unrotated logical units. Draw the background once,
/// followed by all foreground shapes, clipped to `[0,width] x [0,height]`.
/// Shapes intersecting an edge are copied from adjacent lattice cells before
/// clipping. In particular both stripe halves are present across the Y seam.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternTile {
    pub size: [f64; 2],
    pub background: Color,
    pub foreground: Color,
    pub shapes: Vec<Geometry>,
}
#[cfg(feature = "raster")]
#[derive(Clone, Debug)]
pub struct PatternRasterTile {
    pub pixmap: tiny_skia::Pixmap,
    pub logical_size: [f64; 2],
    /// SVG/Canvas affine order `[a,b,c,d,e,f]`, mapping tile pixels to Scene.
    /// Compose the caller's view/DPR transform after this transform.
    pub transform: [f64; 6],
}
impl PatternFill {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.spacing.is_finite() && (0.25..=1024.).contains(&self.spacing),
            "pattern spacing must be in 0.25..=1024 logical units",
        )?;
        let size = match self.shape {
            PatternShape::Dots { diameter, .. } => diameter,
            PatternShape::Squares { size, .. } => size,
            PatternShape::Lines { width } => width,
        };
        ensure(
            size.is_finite() && size >= 0. && size <= self.spacing,
            "pattern motif size must be finite and in 0..=spacing",
        )?;
        ensure(
            self.rotation.is_finite()
                && self.rotation.abs() <= 360_000.
                && self
                    .offset
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "invalid pattern rotation or offset",
        )
    }
    fn size(&self) -> [f64; 2] {
        let stagger = matches!(
            self.shape,
            PatternShape::Dots { stagger: true, .. } | PatternShape::Squares { stagger: true, .. }
        );
        let side = self.spacing * if stagger { 2. } else { 1. };
        [side, side]
    }
    fn rotation(&self) -> [f64; 2] {
        let angle = self.rotation.rem_euclid(360.);
        match angle {
            0. => [1., 0.],
            90. => [0., 1.],
            180. => [-1., 0.],
            270. => [0., -1.],
            _ => {
                let (sin, cos) = (angle * PI / 180.).sin_cos();
                [cos, sin]
            }
        }
    }
    /// Exact bit identity; signed zero has one encoding. No application IDs,
    /// CSS strings or other untrusted text are interpolated into definitions.
    pub fn deterministic_id(&self) -> String {
        let mut id = String::from("kartis-pattern-");
        let size = match self.shape {
            PatternShape::Dots { diameter, stagger } => {
                write!(id, "d{}", u8::from(stagger)).unwrap();
                diameter
            }
            PatternShape::Squares { size, stagger } => {
                write!(id, "s{}", u8::from(stagger)).unwrap();
                size
            }
            PatternShape::Lines { width } => {
                id.push('l');
                width
            }
        };
        for value in [
            size,
            self.spacing,
            self.rotation,
            self.offset[0],
            self.offset[1],
        ] {
            write!(id, "{:016x}", if value == 0. { 0 } else { value.to_bits() }).unwrap();
        }
        for Color(r, g, b, a) in [self.foreground, self.background] {
            write!(id, "-{r:02x}{g:02x}{b:02x}{a:02x}").unwrap();
        }
        id
    }
    pub fn tile(&self) -> Result<PatternTile> {
        self.validate()?;
        let size = self.size();
        let mut shapes = vec![];
        let motif = match self.shape {
            PatternShape::Dots { diameter, .. } => diameter,
            PatternShape::Squares { size, .. } => size,
            PatternShape::Lines { width } => width,
        };
        if motif > 0. {
            if let PatternShape::Lines { width } = self.shape {
                if width == self.spacing {
                    shapes.push(Geometry::Rect {
                        x: 0.,
                        y: 0.,
                        width: size[0],
                        height: size[1],
                    });
                    return Ok(PatternTile {
                        size,
                        background: self.background,
                        foreground: self.foreground,
                        shapes,
                    });
                }
                // Adjacent half-stripes meet exactly after Repeat. Drawing one
                // unwrapped stripe would lose half its area at the tile seam.
                for y in [-width / 2., size[1] - width / 2.] {
                    shapes.push(Geometry::Rect {
                        x: 0.,
                        y,
                        width: size[0],
                        height: width,
                    });
                }
            } else {
                let centers = if size[0] > self.spacing {
                    vec![
                        [self.spacing / 2., self.spacing / 2.],
                        [1.5 * self.spacing, 1.5 * self.spacing],
                    ]
                } else {
                    vec![[self.spacing / 2., self.spacing / 2.]]
                };
                for center in centers {
                    for iy in -1..=1 {
                        for ix in -1..=1 {
                            let x = center[0] + f64::from(ix) * size[0];
                            let y = center[1] + f64::from(iy) * size[1];
                            let h = motif / 2.;
                            if x + h <= 0. || y + h <= 0. || x - h >= size[0] || y - h >= size[1] {
                                continue;
                            }
                            shapes.push(match self.shape {
                                PatternShape::Dots { .. } => Geometry::Circle { x, y, radius: h },
                                _ => Geometry::Rect {
                                    x: x - h,
                                    y: y - h,
                                    width: motif,
                                    height: motif,
                                },
                            });
                        }
                    }
                }
            }
        }
        Ok(PatternTile {
            size,
            background: self.background,
            foreground: self.foreground,
            shapes,
        })
    }
    /// Raster density is physical pixels per unrotated logical unit. The caller
    /// includes DPR and its current view scale. Allocation limits are enforced
    /// before converting dimensions or creating a Canvas/Pixmap.
    pub fn tile_pixels(&self, density: f64) -> Result<[u32; 2]> {
        self.validate()?;
        ensure(
            density.is_finite() && density > 0. && density <= 800.,
            "pattern raster density must be in (0,800]",
        )?;
        let size = self.size();
        let w = (size[0] * density).ceil().max(1.);
        let h = (size[1] * density).ceil().max(1.);
        ensure(
            w <= f64::from(MAX_PATTERN_TILE_SIDE)
                && h <= f64::from(MAX_PATTERN_TILE_SIDE)
                && w * h <= MAX_PATTERN_TILE_PIXELS as f64,
            "pattern raster tile exceeds side/pixel budget",
        )?;
        Ok([w as u32, h as u32])
    }
    pub fn tile_operation_cost(&self, density: f64) -> Result<usize> {
        let [w, h] = self.tile_pixels(density)?;
        let count = self.tile()?.shapes.len() as u64 + 1;
        let cost = u64::from(w) * u64::from(h) * count;
        ensure(
            cost <= MAX_PATTERN_TILE_OPERATIONS,
            "pattern tile operation budget exceeded",
        )?;
        usize::try_from(cost).map_err(|_| ChartError("pattern operation count overflow".into()))
    }
    /// Physical tile pixels -> Scene logical coordinates, preserving the exact
    /// pitch even when ceil(tile_size*density) was not an integer. SVG matrix
    /// conventions apply: first rotate the logical lattice, then translate.
    pub fn pixel_transform(&self, pixels: [u32; 2]) -> Result<[f64; 6]> {
        self.validate()?;
        ensure(
            pixels.iter().all(|v| *v > 0 && *v <= MAX_PATTERN_TILE_SIDE)
                && u64::from(pixels[0]) * u64::from(pixels[1]) <= MAX_PATTERN_TILE_PIXELS,
            "invalid pattern tile dimensions",
        )?;
        let [cos, sin] = self.rotation();
        let size = self.size();
        let x = size[0] / f64::from(pixels[0]);
        let y = size[1] / f64::from(pixels[1]);
        Ok([
            cos * x,
            sin * x,
            -sin * y,
            cos * y,
            self.offset[0],
            self.offset[1],
        ])
    }
    /// Unrotated logical pattern units -> Scene logical coordinates.
    pub fn logical_transform(&self) -> Result<[f64; 6]> {
        self.validate()?;
        let [cos, sin] = self.rotation();
        Ok([cos, sin, -sin, cos, self.offset[0], self.offset[1]])
    }
    pub fn svg_definition(&self) -> Result<String> {
        self.svg_definition_with_namespace("")
    }
    pub(crate) fn svg_definition_with_namespace(&self, namespace: &str) -> Result<String> {
        let tile = self.tile()?;
        let [cos, sin] = self.rotation();
        let mut out = format!(
            "<pattern id=\"{namespace}{}\" width=\"{}\" height=\"{}\" patternUnits=\"userSpaceOnUse\" patternContentUnits=\"userSpaceOnUse\" overflow=\"hidden\" patternTransform=\"matrix({cos} {sin} {} {cos} {} {})\">",
            self.deterministic_id(),
            tile.size[0],
            tile.size[1],
            -sin,
            self.offset[0],
            self.offset[1]
        );
        write!(
            out,
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            tile.size[0],
            tile.size[1],
            color(tile.background)
        )
        .unwrap();
        for shape in tile.shapes {
            match shape {
                Geometry::Circle { x, y, radius } => write!(
                    out,
                    "<circle cx=\"{x}\" cy=\"{y}\" r=\"{radius}\" fill=\"{}\"/>",
                    color(tile.foreground)
                )
                .unwrap(),
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => write!(
                    out,
                    "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" fill=\"{}\"/>",
                    color(tile.foreground)
                )
                .unwrap(),
                Geometry::Path(_) => {
                    unreachable!("pattern tiles contain only circle/rectangle primitives")
                }
            }
        }
        out.push_str("</pattern>");
        Ok(out)
    }
    /// CanvasPattern-producing expression. `ctx` is the renderer's fixed local
    /// CanvasRenderingContext2D. Only validated numbers and generated RGBA color
    /// strings enter the JavaScript; this method accepts no user script or HTML.
    pub fn canvas_script(&self, density: f64) -> Result<String> {
        let pixels = self.tile_pixels(density)?;
        self.tile_operation_cost(density)?;
        self.canvas_script_impl(Some(pixels))
    }
    /// Runtime-density version for generated Canvas replay. Reads only the
    /// renderer's fixed `ctx` transform, accepting finite similarity transforms
    /// (uniform scale/rotation/reflection). Tile budgets are checked in the
    /// generated expression before any Canvas allocation. This preserves high
    /// DPI detail without embedding an application script or global DPR value.
    pub fn canvas_script_current_transform(&self) -> Result<String> {
        self.canvas_script_impl(None)
    }
    fn canvas_script_impl(&self, pixels: Option<[u32; 2]>) -> Result<String> {
        let tile = self.tile()?;
        let [cos, sin] = self.rotation();
        let mut out = String::from("(()=>{");
        if let Some([width, height]) = pixels {
            write!(out, "const width={width},height={height};").unwrap();
        } else {
            write!(out,"const m=ctx.getTransform();const density=Math.hypot(m.a,m.b),other=Math.hypot(m.c,m.d);if(!Number.isFinite(density)||density<=0||density>800||Math.abs(density-other)>density*1e-10||Math.abs(m.a*m.c+m.b*m.d)>density*other*1e-10)throw new Error('pattern requires a finite bounded similarity transform');const width=Math.max(1,Math.ceil({}*density)),height=Math.max(1,Math.ceil({}*density));if(width>{}||height>{}||width*height>{}||width*height*{}>{})throw new Error('pattern tile budget exceeded');",
                tile.size[0],tile.size[1],MAX_PATTERN_TILE_SIDE,MAX_PATTERN_TILE_SIDE,
                MAX_PATTERN_TILE_PIXELS,tile.shapes.len()+1,MAX_PATTERN_TILE_OPERATIONS).unwrap();
        }
        write!(out,"const tile=document.createElement('canvas');tile.width=width;tile.height=height;const tileCtx=tile.getContext('2d');if(!tileCtx)throw new Error('pattern canvas unavailable');tileCtx.scale(width/{},height/{});tileCtx.fillStyle='{}';tileCtx.fillRect(0,0,{},{});tileCtx.fillStyle='{}';",
            tile.size[0],tile.size[1],color(tile.background),tile.size[0],tile.size[1],color(tile.foreground)).unwrap();
        for shape in tile.shapes {
            match shape {
                Geometry::Circle { x, y, radius } => write!(
                    out,
                    "tileCtx.beginPath();tileCtx.arc({x},{y},{radius},0,{});tileCtx.fill();",
                    2. * PI
                )
                .unwrap(),
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => write!(out, "tileCtx.fillRect({x},{y},{width},{height});").unwrap(),
                Geometry::Path(_) => {
                    unreachable!("pattern tiles contain only circle/rectangle primitives")
                }
            }
        }
        write!(out,"const pattern=ctx.createPattern(tile,'repeat');if(!pattern)throw new Error('pattern unavailable');const sx={}/width,sy={}/height;pattern.setTransform(new DOMMatrix([{cos}*sx,{sin}*sx,{}*sy,{cos}*sy,{},{}]));return pattern;}})()",
            tile.size[0],tile.size[1],-sin,self.offset[0],self.offset[1]).unwrap();
        Ok(out)
    }
    #[cfg(feature = "raster")]
    pub fn raster_tile(&self, density: f64) -> Result<PatternRasterTile> {
        use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};
        let pixels = self.tile_pixels(density)?;
        self.tile_operation_cost(density)?;
        let tile = self.tile()?;
        let mut pixmap = Pixmap::new(pixels[0], pixels[1])
            .ok_or_else(|| ChartError("pattern tile allocation failed".into()))?;
        let Color(r, g, b, a) = tile.background;
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, a));
        let mut paint = Paint::default();
        let Color(r, g, b, a) = tile.foreground;
        paint.set_color_rgba8(r, g, b, a);
        let sx = f64::from(pixels[0]) / tile.size[0];
        let sy = f64::from(pixels[1]) / tile.size[1];
        for shape in tile.shapes {
            let path = match shape {
                Geometry::Circle { x, y, radius } => {
                    // Build cubics in physical units. tiny-skia's from_circle
                    // lowers conics to quadratics before an external transform;
                    // scaling that low-resolution approximation magnifies its
                    // tolerance and changes foreground area at higher DPR.
                    let mut path = PathBuilder::new();
                    for command in (Geometry::Circle { x, y, radius }).commands() {
                        use glory::render::PathCommand;
                        match command {
                            PathCommand::Move(p) => {
                                path.move_to((p.x * sx) as f32, (p.y * sy) as f32)
                            }
                            PathCommand::Cubic { first, second, end } => path.cubic_to(
                                (first.x * sx) as f32,
                                (first.y * sy) as f32,
                                (second.x * sx) as f32,
                                (second.y * sy) as f32,
                                (end.x * sx) as f32,
                                (end.y * sy) as f32,
                            ),
                            PathCommand::Close => path.close(),
                            PathCommand::Line(_) => unreachable!("circle commands are cubic"),
                        }
                    }
                    path.finish()
                }
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => Rect::from_xywh(
                    (x * sx) as f32,
                    (y * sy) as f32,
                    (width * sx) as f32,
                    (height * sy) as f32,
                )
                .map(PathBuilder::from_rect),
                Geometry::Path(_) => {
                    unreachable!("pattern tiles contain only circle/rectangle primitives")
                }
            }
            .ok_or_else(|| ChartError("pattern tile primitive is not representable".into()))?;
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        Ok(PatternRasterTile {
            pixmap,
            logical_size: tile.size,
            transform: self.pixel_transform(pixels)?,
        })
    }
}
fn color(Color(r, g, b, a): Color) -> String {
    format!("rgba({r},{g},{b},{})", f64::from(a) / 255.)
}
