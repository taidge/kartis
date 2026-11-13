use crate::{
    ChartError, Result,
    blend::{BlendMode, BlendPlan, BlendRegion},
    scene::ensure,
};
use tiny_skia::{
    BlendMode as Mode, FilterQuality, Mask, Pixmap, PixmapMut, PixmapPaint, PixmapRef, Transform,
};

pub(super) struct Layers {
    plan: BlendPlan,
    bytes: Vec<u8>,
    active: Option<BlendRegion>,
    group_active: bool,
}

impl Layers {
    pub(super) fn new(plan: BlendPlan) -> Result<Self> {
        let [w, h] = plan.scratch;
        let length = w as usize * h as usize * 4;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| ChartError("raster blend scratch allocation failed".into()))?;
        bytes.resize(length, 0);
        Ok(Self {
            plan,
            bytes,
            active: None,
            group_active: false,
        })
    }

    pub(super) fn allocation_pixels(&self) -> usize {
        self.bytes.len() / 4
    }

    pub(super) fn region(&self, index: usize) -> Option<BlendRegion> {
        self.plan.regions[index]
    }

    pub(super) fn active(&self) -> Option<BlendRegion> {
        self.active
    }

    pub(super) fn begin(&mut self, region: BlendRegion, index: usize) -> Result<()> {
        ensure(
            self.active.is_none(),
            "raster blend source is already active",
        )?;
        self.bytes[..region.pixels() as usize * 4].fill(0);
        self.active = Some(region);
        self.group_active = index >= self.plan.mark_count;
        Ok(())
    }

    pub(super) fn group_active(&self) -> bool {
        self.active.is_some() && self.group_active
    }

    pub(super) fn source(&mut self) -> Option<PixmapMut<'_>> {
        let r = self.active?;
        PixmapMut::from_bytes(
            &mut self.bytes[..r.pixels() as usize * 4],
            r.width,
            r.height,
        )
    }

    pub(super) fn end(
        &mut self,
        pixels: &mut Pixmap,
        mask: Option<&Mask>,
        mode: BlendMode,
        opacity: f64,
        commit: bool,
    ) -> Result<()> {
        let r = self
            .active
            .take()
            .ok_or_else(|| ChartError("raster blend source is not active".into()))?;
        if !commit {
            return Ok(());
        }
        let source =
            PixmapRef::from_bytes(&self.bytes[..r.pixels() as usize * 4], r.width, r.height)
                .ok_or_else(|| ChartError("invalid raster blend pixmap".into()))?;
        pixels.draw_pixmap(
            r.x as i32,
            r.y as i32,
            source,
            &PixmapPaint {
                opacity: opacity as f32,
                blend_mode: native_mode(mode),
                quality: FilterQuality::Nearest,
            },
            Transform::identity(),
            mask,
        );
        Ok(())
    }
}

fn native_mode(mode: BlendMode) -> Mode {
    match mode {
        BlendMode::Normal => Mode::SourceOver,
        BlendMode::Multiply => Mode::Multiply,
        BlendMode::Screen => Mode::Screen,
        BlendMode::Overlay => Mode::Overlay,
        BlendMode::Darken => Mode::Darken,
        BlendMode::Lighten => Mode::Lighten,
        BlendMode::ColorDodge => Mode::ColorDodge,
        BlendMode::ColorBurn => Mode::ColorBurn,
        BlendMode::HardLight => Mode::HardLight,
        BlendMode::SoftLight => Mode::SoftLight,
        BlendMode::Difference => Mode::Difference,
        BlendMode::Exclusion => Mode::Exclusion,
        BlendMode::Hue => Mode::Hue,
        BlendMode::Saturation => Mode::Saturation,
        BlendMode::Color => Mode::Color,
        BlendMode::Luminosity => Mode::Luminosity,
    }
}
