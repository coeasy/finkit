//! Formula kernel dispatcher for the Architecture v3 [`UnifiedExecutor`].
//!
//! This is the first real frontend migration onto the numeric executor. Formula
//! parsing and semantic lowering still happen once at compile time; execution of
//! constants and core numeric unary/binary operators uses only [`KernelId`],
//! physical buffer slots and prebound parameter values.

use crate::execution_plan::KernelId;
use crate::state_arena::StateArena;
use crate::unified_executor::{KernelCall, KernelDispatchError, KernelDispatcher, UnifiedExecutor};

/// Numeric dispatcher for formula constants and core arithmetic/logical operators.
#[derive(Debug, Default, Clone, Copy)]
pub struct FormulaKernelDispatcher;

impl FormulaKernelDispatcher {
    const ERR_UNSUPPORTED_KERNEL: u32 = 1;
    const ERR_ARITY: u32 = 2;
    const ERR_PARAMETER: u32 = 3;

    /// Create the default stateless formula dispatcher.
    pub const fn new() -> Self {
        Self
    }
}

impl KernelDispatcher for FormulaKernelDispatcher {
    fn dispatch(
        &mut self,
        call: KernelCall<'_>,
        buffers: &mut [Vec<f64>],
        _states: &mut StateArena,
    ) -> Result<(), KernelDispatchError> {
        if call.kernel == KernelId::from_static("NUMBER") {
            if !call.inputs.is_empty() {
                return Err(KernelDispatchError::new(Self::ERR_ARITY));
            }
            let value = call
                .parameters
                .first()
                .and_then(|parameter| parameter.as_f64())
                .ok_or_else(|| KernelDispatchError::new(Self::ERR_PARAMETER))?;
            buffers[call.output.0].fill(value);
            return Ok(());
        }

        if call.kernel == KernelId::from_static("CALL:MA")
            || call.kernel == KernelId::from_static("CALL:SMA")
            || call.kernel == KernelId::from_static("CALL:EMA")
            || call.kernel == KernelId::from_static("CALL:WMA")
            || call.kernel == KernelId::from_static("CALL:KAMA")
            || call.kernel == KernelId::from_static("CALL:RSI")
            || call.kernel == KernelId::from_static("CALL:MOM")
            || call.kernel == KernelId::from_static("CALL:ROC")
            || call.kernel == KernelId::from_static("CALL:TRIMA")
            || call.kernel == KernelId::from_static("CALL:STD")
        {
            return dispatch_periodic_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ATR")
            || call.kernel == KernelId::from_static("CALL:NATR")
            || call.kernel == KernelId::from_static("CALL:CCI")
        {
            return dispatch_hlc_periodic_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:BBANDS") {
            return dispatch_bbands_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:MACD") {
            return dispatch_macd_line_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:OBV") {
            return dispatch_obv(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:AD")
            || call.kernel == KernelId::from_static("CALL:ADOSC")
            || call.kernel == KernelId::from_static("CALL:MFI")
        {
            return dispatch_volume_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ZSCORE")
            || call.kernel == KernelId::from_static("CALL:VWMA")
            || call.kernel == KernelId::from_static("CALL:CMF")
            || call.kernel == KernelId::from_static("CALL:FISHER")
            || call.kernel == KernelId::from_static("CALL:FISHER_SIGNAL")
            || call.kernel == KernelId::from_static("CALL:TSI")
            || call.kernel == KernelId::from_static("CALL:CHOP")
            || call.kernel == KernelId::from_static("CALL:KDJ")
            || call.kernel == KernelId::from_static("CALL:KDJ_D")
            || call.kernel == KernelId::from_static("CALL:KDJ_J")
            || call.kernel == KernelId::from_static("CALL:ICHIMOKU_TENKAN")
            || call.kernel == KernelId::from_static("CALL:ICHIMOKU_KIJUN")
            || call.kernel == KernelId::from_static("CALL:SUPERTREND")
            || call.kernel == KernelId::from_static("CALL:VWAP")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_UPPER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_LOWER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_MIDDLE")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_WIDTH")
        {
            return dispatch_modern_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("UNARY:Neg") {
            return unary(call, buffers, |value| -value);
        }
        if call.kernel == KernelId::from_static("UNARY:Not") {
            return unary(call, buffers, |value| if value <= 0.0 { 1.0 } else { 0.0 });
        }

        let op = if call.kernel == KernelId::from_static("BINARY:Add") {
            BinaryKernel::Add
        } else if call.kernel == KernelId::from_static("BINARY:Sub") {
            BinaryKernel::Sub
        } else if call.kernel == KernelId::from_static("BINARY:Mul") {
            BinaryKernel::Mul
        } else if call.kernel == KernelId::from_static("BINARY:Div") {
            BinaryKernel::Div
        } else if call.kernel == KernelId::from_static("BINARY:Mod") {
            BinaryKernel::Mod
        } else if call.kernel == KernelId::from_static("BINARY:Pow") {
            BinaryKernel::Pow
        } else if call.kernel == KernelId::from_static("BINARY:Gt") {
            BinaryKernel::Gt
        } else if call.kernel == KernelId::from_static("BINARY:Lt") {
            BinaryKernel::Lt
        } else if call.kernel == KernelId::from_static("BINARY:Gte") {
            BinaryKernel::Gte
        } else if call.kernel == KernelId::from_static("BINARY:Lte") {
            BinaryKernel::Lte
        } else if call.kernel == KernelId::from_static("BINARY:Eq") {
            BinaryKernel::Eq
        } else if call.kernel == KernelId::from_static("BINARY:Neq") {
            BinaryKernel::Neq
        } else if call.kernel == KernelId::from_static("BINARY:And") {
            BinaryKernel::And
        } else if call.kernel == KernelId::from_static("BINARY:Or") {
            BinaryKernel::Or
        } else if call.kernel == KernelId::from_static("BINARY:Xor") {
            BinaryKernel::Xor
        } else {
            return Err(KernelDispatchError::new(Self::ERR_UNSUPPORTED_KERNEL));
        };
        binary(call, buffers, op)
    }
}

/// Execute a one-series formula function directly into the plan-owned output.
///
/// Formula literals are lowered to `NUMBER` buffers, so the period is read from
/// the first element of the second input buffer. The executor guarantees that
/// live dependencies do not alias the output slot; raw pointers let us keep
/// that invariant without allocating temporary `Vec`s or borrowing the whole
/// arena for the duration of the kernel call.
fn dispatch_periodic_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is_sma = call.kernel == KernelId::from_static("CALL:SMA");
    if if is_sma {
        !(2..=3).contains(&call.inputs.len())
    } else {
        call.inputs.len() != 2
    } {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot
        || period_slot == output_slot
        || call.inputs.iter().skip(2).any(|slot| slot.0 == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let input_len = buffers[input_slot].len();
    let output_len = buffers[output_slot].len();
    if input_len != output_len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, input_len),
            std::slice::from_raw_parts_mut(output_ptr, output_len),
        )
    };

    let result = if is_sma {
        let multiplier = call
            .inputs
            .get(2)
            .map(|slot| scalar_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(1.0);
        sma_formula_into(input, period, multiplier, output)
    } else if call.kernel == KernelId::from_static("CALL:MA") {
        crate::math::moving_avg::sma_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:EMA") {
        crate::math::moving_avg::ema_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:WMA") {
        crate::math::moving_avg::wma_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:KAMA") {
        crate::math::moving_avg::kama_into(input, period, 2, 30, output)
    } else if call.kernel == KernelId::from_static("CALL:RSI") {
        crate::indicators::rsi_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:MOM") {
        crate::indicators::mom_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:TRIMA") {
        crate::math::moving_avg::trima_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:STD") {
        crate::math::rolling_stats::stddev_into(input, period, 1.0, output)
    } else {
        crate::indicators::roc_into(input, period, output)
    };

    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute terminal SMA(X, N[, M]) without materialising argument arrays.
///
/// In the formula language SMA is recursive smoothing, distinct from MA's
/// rolling arithmetic mean. Keep its NaN propagation and first-value seed
/// identical to the compatibility implementation.
fn sma_formula_into(
    input: &[f64],
    period: usize,
    multiplier: f64,
    output: &mut [f64],
) -> crate::error::Result<()> {
    if period == 0 || !multiplier.is_finite() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "SMA parameters".to_string(),
            constraint: "period must be > 0 and multiplier must be finite".to_string(),
        });
    }
    output.fill(f64::NAN);
    let denominator = period as f64;
    let retained = denominator - multiplier;
    let mut previous = None;
    for (index, &current) in input.iter().enumerate() {
        if current.is_nan() {
            continue;
        }
        let value = previous
            .map(|old| (multiplier * current + retained * old) / denominator)
            .unwrap_or(current);
        output[index] = value;
        previous = Some(value);
    }
    Ok(())
}

fn period_from_slot(buffers: &[Vec<f64>], slot: usize) -> Result<usize, KernelDispatchError> {
    let value = buffers
        .get(slot)
        .and_then(|values| values.first())
        .copied()
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    Ok(value as usize)
}

#[inline]
fn scalar_from_slot(buffers: &[Vec<f64>], slot: usize) -> Result<f64, KernelDispatchError> {
    buffers
        .get(slot)
        .and_then(|values| values.first())
        .copied()
        .filter(|value| value.is_finite())
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute ATR directly from the HLC dependency slots.
fn dispatch_hlc_periodic_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 4 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let period_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot, period_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let high_ptr = buffers[high_slot].as_ptr();
    let low_ptr = buffers[low_slot].as_ptr();
    let close_ptr = buffers[close_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (high, low, close, output) = unsafe {
        (
            std::slice::from_raw_parts(high_ptr, len),
            std::slice::from_raw_parts(low_ptr, len),
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    let result = if call.kernel == KernelId::from_static("CALL:ATR") {
        crate::indicators::volatility::atr_into(high, low, close, period, output)
    } else if call.kernel == KernelId::from_static("CALL:NATR") {
        crate::indicators::volatility::natr_into(high, low, close, period, output)
    } else {
        crate::indicators::momentum::cci_into(high, low, close, period, output)
    };
    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute formula BOLL/BBANDS as its public upper-band projection.
fn dispatch_bbands_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=3).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || period_slot == output_slot {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let nb_dev = if let Some(parameter) = call.inputs.get(2) {
        let values = &buffers[parameter.0];
        let value = values
            .first()
            .copied()
            .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        if !value.is_finite() {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
        value
    } else {
        2.0
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::math::rolling_stats::bbands_upper_into(input, period, nb_dev, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute formula MACD as its historical MACD/DIF-line projection.
fn dispatch_macd_line_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=4).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || call.inputs.iter().skip(1).any(|slot| slot.0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let fast = period_from_slot(buffers, call.inputs[1].0)?;
    let slow = if let Some(slot) = call.inputs.get(2) {
        period_from_slot(buffers, slot.0)?
    } else {
        26
    };
    let signal = if let Some(slot) = call.inputs.get(3) {
        period_from_slot(buffers, slot.0)?
    } else {
        9
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::indicators::macd_line_into(input, fast, slow, signal, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute OBV using the canonical caller-owned volume kernel.
fn dispatch_obv(call: KernelCall<'_>, buffers: &mut [Vec<f64>]) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let close_slot = call.inputs[0].0;
    let volume_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if close_slot == output_slot || volume_slot == output_slot {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let len = buffers[output_slot].len();
    if buffers[close_slot].len() != len || buffers[volume_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let close_ptr = buffers[close_slot].as_ptr();
    let volume_ptr = buffers[volume_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (close, volume, output) = unsafe {
        (
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts(volume_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::math::volume_kernels::obv_into(close, volume, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute AD/ADOSC/MFI from the canonical caller-owned volume kernels.
fn dispatch_volume_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is_ad = call.kernel == KernelId::from_static("CALL:AD");
    let is_adosc = call.kernel == KernelId::from_static("CALL:ADOSC");
    let valid_arity = if is_ad {
        call.inputs.len() == 4
    } else if is_adosc {
        (4..=6).contains(&call.inputs.len())
    } else {
        call.inputs.len() == 5
    };
    if !valid_arity {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let volume_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot, volume_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
        || call.inputs.iter().skip(4).any(|slot| slot.0 == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
        || buffers[volume_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let high_ptr = buffers[high_slot].as_ptr();
    let low_ptr = buffers[low_slot].as_ptr();
    let close_ptr = buffers[close_slot].as_ptr();
    let volume_ptr = buffers[volume_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (high, low, close, volume, output) = unsafe {
        (
            std::slice::from_raw_parts(high_ptr, len),
            std::slice::from_raw_parts(low_ptr, len),
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts(volume_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };

    let result = if is_ad {
        crate::math::volume_kernels::ad_into(high, low, close, volume, output)
    } else if is_adosc {
        let fast = call
            .inputs
            .get(4)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let slow = call
            .inputs
            .get(5)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(10);
        crate::math::volume_kernels::adosc_into(high, low, close, volume, fast, slow, output)
    } else {
        let period = period_from_slot(buffers, call.inputs[4].0)?;
        crate::math::mfi::mfi_into(high, low, close, volume, period, output)
    };
    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute the formula catalogue's popular non-TA-Lib indicators through the
/// same hot-plan ABI as the TA-Lib-compatible kernels.  The two most common
/// rolling transforms (ZSCORE and VWMA) write directly into the plan buffer;
/// composite indicators reuse their canonical public kernels and copy only the
/// selected projection into the caller-owned output.
fn dispatch_modern_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is = |name| call.kernel == KernelId::from_static(name);
    let arity_ok = if is("CALL:VWAP") {
        call.inputs.len() == 2 || call.inputs.len() == 4
    } else if is("CALL:VWMA") {
        call.inputs.len() == 3
    } else if is("CALL:ZSCORE") {
        call.inputs.len() == 2
    } else if is("CALL:CMF") {
        call.inputs.len() == 5
    } else if is("CALL:CHOP") {
        call.inputs.len() == 4
    } else if is("CALL:FISHER") || is("CALL:FISHER_SIGNAL") {
        call.inputs.len() == 3
    } else if is("CALL:TSI") {
        (2..=3).contains(&call.inputs.len())
    } else if is("CALL:KDJ") || is("CALL:KDJ_D") || is("CALL:KDJ_J") {
        (3..=6).contains(&call.inputs.len())
    } else if is("CALL:SUPERTREND") {
        (4..=5).contains(&call.inputs.len())
    } else {
        call.inputs.len() == 3
    };
    if !arity_ok {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let output_slot = call.output.0;
    let len = buffers
        .get(output_slot)
        .map(Vec::len)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    for input in call.inputs {
        if input.0 == output_slot
            || buffers
                .get(input.0)
                .is_none_or(|values| values.len() != len)
        {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
    }

    if is("CALL:ZSCORE") {
        let period = period_from_slot(buffers, call.inputs[1].0)?;
        let input_ptr = buffers[call.inputs[0].0].as_ptr();
        let output_ptr = buffers[output_slot].as_mut_ptr();
        let (input, output) = unsafe {
            (
                std::slice::from_raw_parts(input_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::indicators::statistics::zscore_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    if is("CALL:VWMA") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let input_ptr = buffers[call.inputs[0].0].as_ptr();
        let volume_ptr = buffers[call.inputs[1].0].as_ptr();
        let output_ptr = buffers[output_slot].as_mut_ptr();
        let (input, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(input_ptr, len),
                std::slice::from_raw_parts(volume_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::math::moving_avg::vwma_into(input, volume, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    if is("CALL:VWAP") {
        let output_ptr = buffers[output_slot].as_mut_ptr();
        if call.inputs.len() == 2 {
            let price_ptr = buffers[call.inputs[0].0].as_ptr();
            let volume_ptr = buffers[call.inputs[1].0].as_ptr();
            let (price, volume, output) = unsafe {
                (
                    std::slice::from_raw_parts(price_ptr, len),
                    std::slice::from_raw_parts(volume_ptr, len),
                    std::slice::from_raw_parts_mut(output_ptr, len),
                )
            };
            let mut price_volume = 0.0;
            let mut total_volume = 0.0;
            for index in 0..len {
                price_volume = price[index].mul_add(volume[index], price_volume);
                total_volume += volume[index];
                output[index] = if total_volume.abs() > 1e-15 {
                    price_volume / total_volume
                } else {
                    f64::NAN
                };
            }
            return Ok(());
        }
        let high_ptr = buffers[call.inputs[0].0].as_ptr();
        let low_ptr = buffers[call.inputs[1].0].as_ptr();
        let close_ptr = buffers[call.inputs[2].0].as_ptr();
        let volume_ptr = buffers[call.inputs[3].0].as_ptr();
        let (high, low, close, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(high_ptr, len),
                std::slice::from_raw_parts(low_ptr, len),
                std::slice::from_raw_parts(close_ptr, len),
                std::slice::from_raw_parts(volume_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::math::volume_kernels::vwap_into(high, low, close, volume, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    let copy_result = |output: &mut [f64], result: &[f64]| {
        if result.len() == output.len() {
            output.copy_from_slice(result);
            Ok(())
        } else {
            Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ))
        }
    };

    if is("CALL:CMF") {
        let period = period_from_slot(buffers, call.inputs[4].0)?;
        let (high, low, close, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[3].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::volume_ext::cmf(high, low, close, volume, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:CHOP") {
        let period = period_from_slot(buffers, call.inputs[3].0)?;
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::chop(high, low, close, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:FISHER") || is("CALL:FISHER_SIGNAL") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let (high, low, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::fisher(high, low, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let selected = if is("CALL:FISHER") {
            result.fisher.as_slice().unwrap()
        } else {
            result.signal.as_slice().unwrap()
        };
        return copy_result(output, selected);
    }

    if is("CALL:TSI") {
        let long_period = period_from_slot(buffers, call.inputs[1].0)?;
        let short_period = call
            .inputs
            .get(2)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(13);
        let (input, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::tsi(input, long_period, short_period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:KDJ") || is("CALL:KDJ_D") || is("CALL:KDJ_J") {
        let n = call
            .inputs
            .get(3)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(9);
        let m1 = call
            .inputs
            .get(4)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let m2 = call
            .inputs
            .get(5)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::china::kdj(high, low, close, n, m1, m2)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let selected = if is("CALL:KDJ_D") {
            result.d.as_slice().unwrap()
        } else if is("CALL:KDJ_J") {
            result.j.as_slice().unwrap()
        } else {
            result.k.as_slice().unwrap()
        };
        return copy_result(output, selected);
    }

    if is("CALL:ICHIMOKU_TENKAN") || is("CALL:ICHIMOKU_KIJUN") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let (high, low, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let upper = crate::math::statistics::rolling_max(high, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let lower = crate::math::statistics::rolling_min(low, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        for index in 0..len {
            output[index] = if upper[index].is_finite() && lower[index].is_finite() {
                (upper[index] + lower[index]) * 0.5
            } else {
                f64::NAN
            };
        }
        return Ok(());
    }

    if is("CALL:SUPERTREND") {
        let period = period_from_slot(buffers, call.inputs[3].0)?;
        let multiplier = call
            .inputs
            .get(4)
            .map(|slot| scalar_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3.0);
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result =
            crate::indicators::supertrend::supertrend(high, low, close, period, multiplier)
                .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.trend_line.as_slice().unwrap());
    }

    let period = period_from_slot(buffers, call.inputs[2].0)?;
    let (high, low, output) = unsafe {
        (
            std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
            std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
            std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
        )
    };
    let result = crate::indicators::donchian::donchian(high, low, period)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    let selected = if is("CALL:DONCHIAN_UPPER") {
        result.upper.as_slice().unwrap()
    } else if is("CALL:DONCHIAN_LOWER") {
        result.lower.as_slice().unwrap()
    } else if is("CALL:DONCHIAN_WIDTH") {
        result.width.as_slice().unwrap()
    } else {
        result.middle.as_slice().unwrap()
    };
    copy_result(output, selected)
}

#[derive(Debug, Clone, Copy)]
enum BinaryKernel {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
    And,
    Or,
    Xor,
}

fn unary(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: impl Fn(f64) -> f64,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 1 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input = call.inputs[0].0;
    let output = call.output.0;
    let len = buffers[output].len();
    for index in 0..len {
        let value = buffers[input][index];
        buffers[output][index] = op(value);
    }
    Ok(())
}

fn binary(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: BinaryKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let left = call.inputs[0].0;
    let right = call.inputs[1].0;
    let output = call.output.0;
    let len = buffers[output].len();
    for index in 0..len {
        let lhs = buffers[left][index];
        let rhs = buffers[right][index];
        buffers[output][index] = apply_binary(op, lhs, rhs);
    }
    Ok(())
}

#[inline]
fn apply_binary(op: BinaryKernel, lhs: f64, rhs: f64) -> f64 {
    match op {
        BinaryKernel::Add => lhs + rhs,
        BinaryKernel::Sub => lhs - rhs,
        BinaryKernel::Mul => lhs * rhs,
        BinaryKernel::Div => {
            if rhs.abs() < 1e-15 {
                f64::NAN
            } else {
                lhs / rhs
            }
        }
        BinaryKernel::Mod => {
            if rhs.abs() < 1e-15 {
                f64::NAN
            } else {
                lhs - (lhs / rhs).floor() * rhs
            }
        }
        BinaryKernel::Pow => lhs.powf(rhs),
        BinaryKernel::Gt => bool_value(lhs > rhs),
        BinaryKernel::Lt => bool_value(lhs < rhs),
        BinaryKernel::Gte => bool_value(lhs >= rhs),
        BinaryKernel::Lte => bool_value(lhs <= rhs),
        BinaryKernel::Eq => bool_value((lhs - rhs).abs() < 1e-10),
        BinaryKernel::Neq => bool_value((lhs - rhs).abs() >= 1e-10),
        BinaryKernel::And => bool_value(lhs > 0.0 && rhs > 0.0),
        BinaryKernel::Or => bool_value(lhs > 0.0 || rhs > 0.0),
        BinaryKernel::Xor => bool_value((lhs > 0.0) != (rhs > 0.0)),
    }
}

#[inline]
const fn bool_value(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// Construct a reusable unified executor for one compiled formula hot plan.
pub fn unified_formula_executor(
    plan: &super::hot_plan::FormulaHotPlan,
) -> UnifiedExecutor<FormulaKernelDispatcher> {
    UnifiedExecutor::new(plan.hot().clone(), FormulaKernelDispatcher::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{parse_formula, FormulaHotPlan};

    #[test]
    fn real_formula_frontend_executes_constant_arithmetic_on_unified_executor() {
        let ast = parse_formula("CLOSE * 2 + 1").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.5, -3.0, 0.0];

        let result = executor.execute(&[&close]).unwrap();
        assert_eq!(result.values, vec![vec![3.0, 6.0, -5.0, 1.0]]);
    }

    #[test]
    fn numeric_semantics_match_legacy_formula_contract() {
        let ast = parse_formula("(CLOSE / 0) + (CLOSE > 1)").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.0];

        let result = executor.execute(&[&close]).unwrap();
        assert!(result.values[0][0].is_nan());
        assert!(result.values[0][1].is_nan());
    }

    #[test]
    fn range_and_last_use_the_same_formula_dispatcher() {
        let ast = parse_formula("-CLOSE + 10").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [2.0, 4.0, 8.0];

        let range = executor.execute_range(&[&close], 0..2).unwrap();
        assert_eq!(range.values, vec![vec![8.0, 6.0]]);
        let last = executor.execute_last(&[&close]).unwrap();
        assert_eq!(last.values, vec![vec![2.0]]);
    }

    #[test]
    fn canonical_indicator_calls_run_inside_the_unified_formula_executor() {
        let ast = parse_formula("EMA(CLOSE, 3) + ROC(CLOSE, 2)").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.0, 4.0, 8.0, 16.0];

        let result = executor.execute(&[&close]).unwrap();
        assert!(result.values[0][0].is_nan());
        assert!(result.values[0][1].is_nan());
        assert!(result.values[0][2].is_finite());
        assert!(result.values[0][4].is_finite());
    }

    #[test]
    fn extended_indicator_calls_use_canonical_zero_copy_kernels() {
        let high = [10.0, 10.5, 11.0, 10.8, 11.4, 11.8, 12.1, 11.9, 12.4, 12.7];
        let low = [9.0, 9.5, 10.0, 9.7, 10.3, 10.8, 11.0, 10.9, 11.3, 11.6];
        let close = [9.5, 10.2, 10.7, 10.1, 11.0, 11.5, 11.7, 11.2, 12.0, 12.4];
        let volume = [
            100.0, 110.0, 120.0, 105.0, 130.0, 125.0, 140.0, 115.0, 150.0, 145.0,
        ];

        fn execute(
            source: &str,
            high: &[f64],
            low: &[f64],
            close: &[f64],
            volume: &[f64],
        ) -> Vec<f64> {
            let ast = parse_formula(source).unwrap();
            let plan = FormulaHotPlan::compile(&ast).unwrap();
            let mut inputs = vec![close; plan.hot().input_layout().len()];
            for (name, values) in [
                ("VARIABLE:HIGH", high),
                ("VARIABLE:LOW", low),
                ("VARIABLE:CLOSE", close),
                ("VARIABLE:VOLUME", volume),
            ] {
                if let Some(slot) = plan.hot().input_layout().slot_for_operation(name) {
                    inputs[slot.0] = values;
                }
            }
            let mut executor = unified_formula_executor(&plan);
            executor.execute(&inputs).unwrap().values[0].clone()
        }

        let cases = [
            (
                "CCI(HIGH,LOW,CLOSE,3)",
                crate::indicators::momentum::cci(&high, &low, &close, 3).unwrap(),
            ),
            (
                "NATR(HIGH,LOW,CLOSE,3)",
                crate::indicators::volatility::natr(&high, &low, &close, 3).unwrap(),
            ),
            (
                "TRIMA(CLOSE,3)",
                crate::math::moving_avg::trima(&close, 3).unwrap(),
            ),
            (
                "AD(HIGH,LOW,CLOSE,VOLUME)",
                crate::math::volume_kernels::ad(&high, &low, &close, &volume).unwrap(),
            ),
            (
                "ADOSC(HIGH,LOW,CLOSE,VOLUME,3,5)",
                crate::math::volume_kernels::adosc(&high, &low, &close, &volume, 3, 5).unwrap(),
            ),
            (
                "MFI(HIGH,LOW,CLOSE,VOLUME,3)",
                crate::math::mfi::mfi(&high, &low, &close, &volume, 3).unwrap(),
            ),
        ];

        for (source, expected) in cases {
            let actual = execute(source, &high, &low, &close, &volume);
            assert_eq!(actual.len(), expected.len(), "{source}");
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (*actual - *expected).abs() < 1e-10,
                    "{source}: actual={actual:?} expected={expected:?}"
                );
            }
        }

        let modern_cases = [
            (
                "ZSCORE(CLOSE,3)",
                crate::indicators::statistics::zscore(&close, 3).unwrap(),
            ),
            (
                "VWMA(CLOSE,VOLUME,3)",
                crate::math::moving_avg::vwma(&close, &volume, 3).unwrap(),
            ),
            (
                "CMF(HIGH,LOW,CLOSE,VOLUME,3)",
                crate::indicators::volume_ext::cmf(&high, &low, &close, &volume, 3).unwrap(),
            ),
            (
                "FISHER(HIGH,LOW,3)",
                crate::indicators::momentum_ext::fisher(&high, &low, 3)
                    .unwrap()
                    .fisher,
            ),
            (
                "FISHER_SIGNAL(HIGH,LOW,3)",
                crate::indicators::momentum_ext::fisher(&high, &low, 3)
                    .unwrap()
                    .signal,
            ),
            (
                "TSI(CLOSE,4,2)",
                crate::indicators::momentum_ext::tsi(&close, 4, 2).unwrap(),
            ),
            (
                "CHOP(HIGH,LOW,CLOSE,3)",
                crate::indicators::momentum_ext::chop(&high, &low, &close, 3).unwrap(),
            ),
            (
                "KDJ_D(HIGH,LOW,CLOSE,3,2,2)",
                crate::indicators::china::kdj(&high, &low, &close, 3, 2, 2)
                    .unwrap()
                    .d,
            ),
            (
                "ICHIMOKU_TENKAN(HIGH,LOW,3)",
                crate::math::statistics::rolling_max(&high, 3)
                    .unwrap()
                    .iter()
                    .zip(
                        crate::math::statistics::rolling_min(&low, 3)
                            .unwrap()
                            .iter(),
                    )
                    .map(|(upper, lower)| {
                        if upper.is_finite() && lower.is_finite() {
                            (upper + lower) * 0.5
                        } else {
                            f64::NAN
                        }
                    })
                    .collect(),
            ),
            (
                "SUPERTREND(HIGH,LOW,CLOSE,3,2)",
                crate::indicators::supertrend::supertrend(&high, &low, &close, 3, 2.0)
                    .unwrap()
                    .trend_line,
            ),
            (
                "DONCHIAN_WIDTH(HIGH,LOW,3)",
                crate::indicators::donchian::donchian(&high, &low, 3)
                    .unwrap()
                    .width,
            ),
        ];

        for (source, expected) in modern_cases {
            let actual = execute(source, &high, &low, &close, &volume);
            assert_eq!(actual.len(), expected.len(), "{source}");
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (*actual - *expected).abs() < 1e-10,
                    "{source}: actual={actual:?} expected={expected:?}"
                );
            }
        }

        let actual = execute("SMA(CLOSE,3,1)", &high, &low, &close, &volume);
        let mut previous = None;
        for (index, &value) in close.iter().enumerate() {
            let expected = previous
                .map(|old| (value + 2.0 * old) / 3.0)
                .unwrap_or(value);
            assert!((actual[index] - expected).abs() < 1e-12);
            previous = Some(expected);
        }
    }
}
