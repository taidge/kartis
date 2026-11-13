//! Bounded fdlibm trigonometry for reproducible RGBA8 rounding.
//!
//! Adapted from Netlib fdlibm k_sin.c, k_cos.c, and e_rem_pio2.c.
//!
//! Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//!
//! Developed at SunSoft, a Sun Microsystems, Inc. business.
//! Permission to use, copy, modify, and distribute this
//! software is freely granted, provided that this notice
//! is preserved.
//!
//! The caller supplies angles from the named Cubehelix or Sinebow profiles,
//! all finite and smaller than 8 radians. Keeping the argument reduction and
//! polynomial evaluation in Rust avoids platform libm differences at half
//! channel values, such as Sinebow's red at one-quarter turn.

const SIN: [f64; 6] = [
    f64::from_bits(0xbfc5_5555_5555_5549),
    f64::from_bits(0x3f81_1111_1110_f8a6),
    f64::from_bits(0xbf2a_01a0_19c1_61d5),
    f64::from_bits(0x3ec7_1de3_57b1_fe7d),
    f64::from_bits(0xbe5a_e5e6_8a2b_9ceb),
    f64::from_bits(0x3de5_d93a_5acf_d57c),
];
const COS: [f64; 6] = [
    f64::from_bits(0x3fa5_5555_5555_554c),
    f64::from_bits(0xbf56_c16c_16c1_5177),
    f64::from_bits(0x3efa_01a0_19cb_1590),
    f64::from_bits(0xbe92_7e4f_809c_52ad),
    f64::from_bits(0x3e21_ee9e_bdb4_b1c4),
    f64::from_bits(0xbda8_fae9_be88_38d4),
];

fn magnitude_word(x: f64) -> u32 {
    ((x.to_bits() >> 32) as u32) & 0x7fff_ffff
}

// Nearest multiple of pi/2, plus the high/low parts of the remainder.
fn reduce(x: f64) -> (i32, f64, f64) {
    debug_assert!((0. ..8.).contains(&x));
    if magnitude_word(x) <= 0x3fe9_21fb {
        return (0, x, 0.);
    }
    const TWO_OVER_PI: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883);
    const HALF_PI_HIGH: f64 = f64::from_bits(0x3ff9_21fb_5440_0000);
    const HALF_PI_LOW: f64 = f64::from_bits(0x3dd0_b461_1a62_6331);
    let quadrant = (x * TWO_OVER_PI + 0.5) as i32;
    let n = f64::from(quadrant);
    let mut high = x - n * HALF_PI_HIGH;
    let mut low = n * HALF_PI_LOW;
    let exponent = (magnitude_word(x) >> 20) as i32;
    // A further split is needed only near cancellation at multiples of pi/2.
    for (bits, part, tail) in [
        (16, 0x3dd0_b461_1a60_0000, 0x3ba3_198a_2e03_7073),
        (49, 0x3ba3_198a_2e00_0000, 0x397b_839a_2520_49c1),
    ] {
        if exponent - (magnitude_word(high - low) >> 20) as i32 <= bits {
            break;
        }
        let previous = high;
        let correction = n * f64::from_bits(part);
        high = previous - correction;
        low = n * f64::from_bits(tail) - ((previous - high) - correction);
    }
    let head = high - low;
    (quadrant, head, (high - head) - low)
}

fn polynomial(coefficients: &[f64], x: f64) -> f64 {
    coefficients.iter().rev().fold(0., |value, c| c + x * value)
}

fn sin_kernel(x: f64, tail: f64, reduced: bool) -> f64 {
    if magnitude_word(x) < 0x3e40_0000 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = polynomial(&SIN[1..], z);
    if reduced {
        x - ((z * (0.5 * tail - v * r) - tail) - v * SIN[0])
    } else {
        x + v * (SIN[0] + z * r)
    }
}

fn cos_kernel(x: f64, tail: f64) -> f64 {
    let word = magnitude_word(x);
    if word < 0x3e40_0000 {
        return 1.;
    }
    let z = x * x;
    let remainder = z * (z * polynomial(&COS, z)) - x * tail;
    if word < 0x3fd3_3333 {
        1. - (0.5 * z - remainder)
    } else {
        let quarter = if word > 0x3fe9_0000 {
            0.28125
        } else {
            f64::from_bits(u64::from(word - 0x0020_0000) << 32)
        };
        (1. - quarter) - ((0.5 * z - quarter) - remainder)
    }
}

pub(super) fn sin(angle: f64) -> f64 {
    let (quadrant, x, tail) = reduce(angle.abs());
    let value = match quadrant & 3 {
        0 => sin_kernel(x, tail, quadrant != 0),
        1 => cos_kernel(x, tail),
        2 => -sin_kernel(x, tail, true),
        _ => -cos_kernel(x, tail),
    };
    angle.signum() * value
}

pub(super) fn sin_cos(angle: f64) -> (f64, f64) {
    let (quadrant, x, tail) = reduce(angle.abs());
    let sin = sin_kernel(x, tail, quadrant != 0);
    let cos = cos_kernel(x, tail);
    let (sin, cos) = match quadrant & 3 {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    };
    (angle.signum() * sin, cos)
}
