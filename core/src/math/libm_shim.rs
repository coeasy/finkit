//! no_std-compatible floating-point math helpers.
//!
//! `core::math` is the crate's designated `no_std`-capable subset (it is the
//! only top-level module compiled even when the `std` feature is off). To keep
//! the numeric helpers portable to bare-metal / `no_std` targets, every float
//! primitive they need is routed through this module instead of reaching for
//! `std` directly.
//!
//! * In `std` builds the helpers delegate to the `f64`/`f32` intrinsic methods
//!   (which live in `core` and are therefore available either way).
//! * In `no_std` builds they delegate to the [`libm`] crate.
//!
//! This module is the single source of truth for the `f64_*` free functions
//! previously duplicated inside `simd_ops.rs`, plus the `FloatExt` trait used
//! by the isolated numeric helpers.

#![allow(dead_code)]

#[cfg(not(feature = "std"))]
use libm::{
    atan as libm_atan, atan2 as libm_atan2, ceil as libm_ceil, cos as libm_cos, exp as libm_exp,
    fabs as libm_abs, floor as libm_floor, fmax as libm_max, fmin as libm_min, log as libm_ln,
    pow as libm_powf, round as libm_round, sin as libm_sin, sqrt as libm_sqrt, trunc as libm_trunc,
};

/// `core::f64::consts::PI` re-exported so callers need not reference `std`.
pub const FM_PI: f64 = core::f64::consts::PI;
/// `core::f64::consts::FRAC_PI_2` re-exported for `no_std` portability.
pub const FM_FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;

/// `no_std`-portable floating-point primitives.
///
/// Implemented for `f64` (the crate's working precision). Each method resolves
/// to a `core` intrinsic under `std` and to a `libm` call under `no_std`.
pub trait FloatExt {
    fn fm_sqrt(self) -> f64;
    fn fm_ln(self) -> f64;
    fn fm_exp(self) -> f64;
    fn fm_sin(self) -> f64;
    fn fm_cos(self) -> f64;
    fn fm_atan(self) -> f64;
    fn fm_atan2(self, other: f64) -> f64;
    fn fm_powf(self, exp: f64) -> f64;
    fn fm_abs(self) -> f64;
    fn fm_floor(self) -> f64;
    fn fm_ceil(self) -> f64;
    fn fm_round(self) -> f64;
    fn fm_trunc(self) -> f64;
    fn fm_min(self, other: f64) -> f64;
    fn fm_max(self, other: f64) -> f64;
}

impl FloatExt for f64 {
    #[inline]
    fn fm_sqrt(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.sqrt()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_sqrt(self)
        }
    }

    #[inline]
    fn fm_ln(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.ln()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_ln(self)
        }
    }

    #[inline]
    fn fm_exp(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.exp()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_exp(self)
        }
    }

    #[inline]
    fn fm_sin(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.sin()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_sin(self)
        }
    }

    #[inline]
    fn fm_cos(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.cos()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_cos(self)
        }
    }

    #[inline]
    fn fm_atan(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.atan()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_atan(self)
        }
    }

    #[inline]
    fn fm_atan2(self, other: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            self.atan2(other)
        }
        #[cfg(not(feature = "std"))]
        {
            libm_atan2(self, other)
        }
    }

    #[inline]
    fn fm_powf(self, exp: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            self.powf(exp)
        }
        #[cfg(not(feature = "std"))]
        {
            libm_powf(self, exp)
        }
    }

    #[inline]
    fn fm_abs(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.abs()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_abs(self)
        }
    }

    #[inline]
    fn fm_floor(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.floor()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_floor(self)
        }
    }

    #[inline]
    fn fm_ceil(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.ceil()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_ceil(self)
        }
    }

    #[inline]
    fn fm_round(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.round()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_round(self)
        }
    }

    #[inline]
    fn fm_trunc(self) -> f64 {
        #[cfg(feature = "std")]
        {
            self.trunc()
        }
        #[cfg(not(feature = "std"))]
        {
            libm_trunc(self)
        }
    }

    #[inline]
    fn fm_min(self, other: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            self.min(other)
        }
        #[cfg(not(feature = "std"))]
        {
            libm_min(self, other)
        }
    }

    #[inline]
    fn fm_max(self, other: f64) -> f64 {
        #[cfg(feature = "std")]
        {
            self.max(other)
        }
        #[cfg(not(feature = "std"))]
        {
            libm_max(self, other)
        }
    }
}

/// Square root — `no_std`-portable. Delegates to `core`/`libm`.
#[inline]
pub fn f64_sqrt(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        libm_sqrt(x)
    }
}

/// Natural logarithm — `no_std`-portable.
#[inline]
pub fn f64_ln(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.ln()
    }
    #[cfg(not(feature = "std"))]
    {
        libm_ln(x)
    }
}

/// Exponentiation (`e^x`) — `no_std`-portable.
#[inline]
pub fn f64_exp(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.exp()
    }
    #[cfg(not(feature = "std"))]
    {
        libm_exp(x)
    }
}

/// Arctangent — `no_std`-portable.
#[inline]
pub fn f64_atan(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.atan()
    }
    #[cfg(not(feature = "std"))]
    {
        libm_atan(x)
    }
}

/// Power — `no_std`-portable.
#[inline]
pub fn f64_powf(x: f64, exp: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.powf(exp)
    }
    #[cfg(not(feature = "std"))]
    {
        libm_powf(x, exp)
    }
}

/// Absolute value — `no_std`-portable.
#[inline]
pub fn f64_abs(x: f64) -> f64 {
    #[cfg(feature = "std")]
    {
        x.abs()
    }
    #[cfg(not(feature = "std"))]
    {
        libm_abs(x)
    }
}
