//! The four formula functions that consume a `STRING_LITERAL` index.
//!
//! `EM_REF`, `BLOCKDATA`, `BLOCKINDEX` and `BLOCKAVG` take a string literal as
//! their first argument. The plan path does not pass text: the `STRING_LITERAL`
//! kernel emits an **absolute** index into `FormulaContext::string_table`, and
//! these functions resolve that index back to a name and then read host data
//! keyed by it. They are the only functions that consume a string index, so
//! this module is the complete string-literal execution surface on the plan
//! path — before it existed, a formula failed at dispatch merely for reading its
//! own literal.
//!
//! The two pieces of host data they need (`ctx.em_data`, `ctx.block_data`) and
//! the table itself travel in [`HostContext`], because none of them can be
//! expressed as a numeric input slot.

use crate::buffer_arena::BufferSlot;
use crate::execution_plan::KernelId;
use crate::formula::types::HostContext;
use crate::unified_executor::{KernelCall, KernelDispatchError};
use ndarray::Array1;

use super::unified_dispatch::FormulaKernelDispatcher;

/// Whether `kernel` belongs to the string-consuming family.
///
/// Kept as a predicate so the dispatcher's match adds one call rather than four
/// `KernelId::from_static` comparisons, and so the family's membership lives
/// next to its implementation.
pub(crate) fn handles(kernel: KernelId) -> bool {
    kernel == KernelId::from_static("CALL:EM_REF")
        || kernel == KernelId::from_static("CALL:BLOCKDATA")
        || kernel == KernelId::from_static("CALL:BLOCKINDEX")
        || kernel == KernelId::from_static("CALL:BLOCKAVG")
}

/// Resolve argument `arg` of a string-consuming call back to its text.
///
/// The `STRING_LITERAL` kernel emits an **absolute** index into the context's
/// table (plan-local index + `string_base`), so this reads that slot exactly as
/// the tree path's `get_string_from_hash` does. An index that is missing,
/// non-finite, negative or out of range is an error rather than a silent read
/// of slot 0 — the tree path reports the same "must be a valid string".
fn resolve_string_index(
    host: &HostContext<'_>,
    buffers: &[Vec<f64>],
    inputs: &[BufferSlot],
    arg: usize,
) -> Result<String, KernelDispatchError> {
    let Some(slot) = inputs.get(arg) else {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    };
    let index = buffers[slot.0].first().copied().unwrap_or(f64::NAN);
    if index.is_nan() || index < 0.0 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let table = host
        .string_table
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    // `index` is finite and non-negative here (checked above) and is a table
    // position, so the cast neither truncates a fractional part nor loses a
    // sign — it is the same `f64 as usize` the tree path performs.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let position = index as usize;
    table
        .get(position)
        .cloned()
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Dispatch one of the four string-consuming calls.
///
/// They mirror their `fn_*` counterparts in `functions_legacy` exactly,
/// including the behaviours that are easy to get wrong:
///
/// * **Absent host data is not an error.** With no `em_data` / `block_data` the
///   tree path returns an all-`NaN` series, so the plan path must too — only a
///   *present* container with a *missing key* is an error.
/// * **A length mismatch yields `NaN`, not a truncation.** The tree path
///   replaces the whole series with `NaN` when the stored series is not the
///   output length, rather than aligning prefixes.
/// * **`EM_REF`'s period goes through `extract_n`.** A zero period is an error
///   rather than a no-op, and a `NaN` slot saturates to zero exactly as the
///   `f64 as usize` cast there does.
pub(crate) fn dispatch(
    host: &HostContext<'_>,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let out = call.output.0;
    let length = buffers[out].len();

    // Match the legacy functions' minimum-arity checks before resolving any
    // literal. The legacy `ensure_args_len` deliberately allows trailing
    // arguments, so do not tighten that public behavior on the plan path.
    let expected_arity = if call.kernel == KernelId::from_static("CALL:EM_REF")
        || call.kernel == KernelId::from_static("CALL:BLOCKDATA")
    {
        2
    } else {
        1
    };
    if call.inputs.len() < expected_arity {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let name = resolve_string_index(host, buffers, call.inputs, 0)?;

    let series: &Array1<f64> = if call.kernel == KernelId::from_static("CALL:EM_REF") {
        // `EM_REF(name, n)` is `REF` over an external series: bar `i` reads bar
        // `i - n` of the named series and the first `n` bars are `NaN`.
        //
        // The cast mirrors `extract_n`: a NaN slot saturates to 0 and a
        // negative one clamps to 0, so the `n == 0` check below rejects both.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = buffers[call.inputs[1].0]
            .first()
            .copied()
            .unwrap_or(f64::NAN) as usize;
        if n == 0 {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
        let Some(em) = host.em_data else {
            buffers[out].fill(f64::NAN);
            return Ok(());
        };
        let data = em
            .external_data
            .get(&name)
            .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let Some(source) = data.as_slice() else {
            buffers[out].fill(f64::NAN);
            return Ok(());
        };
        if source.len() != length {
            buffers[out].fill(f64::NAN);
            return Ok(());
        }
        for index in 0..length {
            buffers[out][index] = if index >= n {
                source[index - n]
            } else {
                f64::NAN
            };
        }
        return Ok(());
    } else {
        let Some(block_data) = host.block_data else {
            buffers[out].fill(f64::NAN);
            return Ok(());
        };
        if call.kernel == KernelId::from_static("CALL:BLOCKDATA") {
            let field = resolve_string_index(host, buffers, call.inputs, 1)?.to_uppercase();
            match field.as_str() {
                "INDEX" | "CLOSE" | "C" => block_data.index_close.get(&name),
                "AVG" | "AVGPRICE" => block_data.avg_price.get(&name),
                "PCT" | "PCTCHANGE" | "CHANGE" => block_data.pct_change.get(&name),
                "VOL" | "VOLUME" | "V" => block_data.volume.get(&name),
                "AMOUNT" | "A" => block_data.amount.get(&name),
                _ => block_data
                    .custom_fields
                    .get(&name)
                    .and_then(|fields| fields.get(&field)),
            }
            .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
        } else {
            // The two single-argument members are fixed-field aliases.
            let field = if call.kernel == KernelId::from_static("CALL:BLOCKINDEX") {
                &block_data.index_close
            } else {
                &block_data.avg_price
            };
            field
                .get(&name)
                .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
        }
    };

    let Some(series) = series.as_slice() else {
        buffers[out].fill(f64::NAN);
        return Ok(());
    };
    if series.len() != length {
        buffers[out].fill(f64::NAN);
        return Ok(());
    }
    buffers[out].copy_from_slice(series);
    Ok(())
}
