//! The plan path's compatibility bridge to the canonical formula functions.
//!
//! Some `CALL:` kernels have no hand-written in-place plan kernel. Rather than
//! duplicate their logic — and risk the two implementations drifting — the plan
//! path delegates them to the exact `canonical_*` function the
//! tree/bytecode/JIT frontends call, so the plan's output is identical by
//! construction. `formula_plan_differential.rs` remains the independent safety
//! net that would catch a delegate whose behaviour differs from what the plan
//! expects.
//!
//! This module owns the bridge's execution surface:
//!
//! * the two name rosters that *select* the bridge ([`TALIB_FORMULA_GAP_KERNELS`]
//!   and [`SCREENING_KERNELS`]),
//! * the bridge executor ([`dispatch_formula_bridge_call`]), and
//! * the delegation context builder ([`bridge_context`]).
//!
//! The name→function tables themselves (`TALIB_081_KERNELS`, `SCREENING_DELEGATES`)
//! stay local to `dispatch_modern_call` in `unified_dispatch`, because they exist
//! only to be iterated there.

use crate::formula::unified_dispatch::FormulaKernelDispatcher;
use crate::unified_executor::{KernelCall, KernelDispatchError};
use ndarray::Array1;

/// The domestic screening / conditional-counting family.
///
/// These are the *canonical* names behind a much larger alias surface: the
/// planner resolves every registry alias to its canonical name
/// (`compute_ir::function_metadata` uses `canonical_name(spec.name)`), so
/// `CROSSUP`, `BULLISH_CROSS`, `VOLSURGE`, `VOLUME_EXPANSION`, `TENKAN`,
/// `BREAKOUT_SCREEN` and the rest never appear as their own `CALL:` node — they
/// all lower to one of the eleven names below. A missing kernel here therefore
/// broke every alias pointing at it at once.
///
/// Unlike [`TALIB_FORMULA_GAP_KERNELS`] these are **not** routed through the
/// compatibility bridge: each has an explicit canonical delegate in
/// `SCREENING_DELEGATES`, which is the same function the tree/bytecode/JIT
/// frontends call, so the plan path's output is identical by construction
/// rather than by agreement.
pub(crate) const SCREENING_KERNELS: &[&str] = &[
    "CALL:BARSLAST",
    "CALL:BREAKDOWN",
    "CALL:BREAKOUT",
    "CALL:COUNT",
    "CALL:DEAD_CROSS",
    "CALL:GAP_SIGNAL",
    "CALL:GOLDEN_CROSS",
    "CALL:MA_ALIGN",
    "CALL:RELATIVE_STRENGTH",
    "CALL:TREND_BREAKOUT",
    "CALL:VOLUME_SURGE",
];

/// TA-Lib functions that already have a formula implementation but do not yet
/// have a hand-written in-place plan kernel. They are routed through the cached
/// formula function table below. This is an intentional compatibility bridge:
/// it closes plan reachability first, while the specialized kernels can later
/// remove the allocation/copy path one family at a time without changing the
/// numeric contract.
pub(crate) const TALIB_FORMULA_GAP_KERNELS: &[&str] = &[
    "CALL:ADXR",
    "CALL:APO",
    "CALL:AROONOSC",
    "CALL:AVGPRICE",
    "CALL:AVGDEV",
    "CALL:BETA",
    "CALL:BOP",
    "CALL:CDL2CROWS",
    "CALL:CDL3BLACKCROWS",
    "CALL:CDL3INSIDE",
    "CALL:CDL3LINESTRIKE",
    "CALL:CDL3OUTSIDE",
    "CALL:CDL3STARSINSOUTH",
    "CALL:CDL3WHITESOLDIERS",
    "CALL:CDLDOJI",
    "CALL:CDLDOJISTAR",
    "CALL:CDLDRAGONFLYDOJI",
    "CALL:CDLENGULFING",
    "CALL:CDLEVENINGDOJISTAR",
    "CALL:CDLGRAVESTONEDOJI",
    "CALL:CDLHAMMER",
    "CALL:CDLHANGINGMAN",
    "CALL:CDLHARAMI",
    "CALL:CDLMARUBOZU",
    "CALL:CDLPIERCING",
    "CALL:CDLSHOOTINGSTAR",
    "CALL:CDLSPINNINGTOP",
    "CALL:CDLABANDONEDBABY",
    "CALL:CDLADVANCEBLOCK",
    "CALL:CDLBELTHOLD",
    "CALL:CDLBREAKAWAY",
    "CALL:CDLCLOSINGMARUBOZU",
    "CALL:CDLCONCEALBABYSWALL",
    "CALL:CDLCOUNTERATTACK",
    "CALL:CDLDARKCLOUDCOVER",
    "CALL:CDLEVENINGSTAR",
    "CALL:CDLGAPSIDESIDEWHITE",
    "CALL:CDLHARAMICROSS",
    "CALL:CDLHIGHWAVE",
    "CALL:CDLHIKKAKE",
    "CALL:CDLHIKKAKEMOD",
    "CALL:CDLHOMINGPIGEON",
    "CALL:CDLIDENTICAL3CROWS",
    "CALL:CDLINNECK",
    "CALL:CDLINVERTEDHAMMER",
    "CALL:CDLKICKING",
    "CALL:CDLKICKINGBYLENGTH",
    "CALL:CDLLADDERBOTTOM",
    "CALL:CDLLONGLEGGEDDOJI",
    "CALL:CDLLONGLINE",
    "CALL:CDLMATCHINGLOW",
    "CALL:CDLMATHOLD",
    "CALL:CDLMORNINGDOJISTAR",
    "CALL:CDLMORNINGSTAR",
    "CALL:CDLONNECK",
    "CALL:CDLRICKSHAWMAN",
    "CALL:CDLRISEFALL3METHODS",
    "CALL:CDLSEPARATINGLINES",
    "CALL:CDLSHORTLINE",
    "CALL:CDLSTALLEDPATTERN",
    "CALL:CDLSTICKSANDWICH",
    "CALL:CDLTAKURI",
    "CALL:CDLTASUKIGAP",
    "CALL:CDLTHRUSTING",
    "CALL:CDLTRISTAR",
    "CALL:CDLUNIQUE3RIVER",
    "CALL:CDLUPSIDEGAP2CROWS",
    "CALL:CDLXSIDEGAP3METHODS",
    "CALL:CMO",
    "CALL:DEMA",
    "CALL:DPO",
    "CALL:DX",
    "CALL:HT_DCPERIOD",
    "CALL:HT_DCPHASE",
    "CALL:HT_PHASOR",
    "CALL:HT_SINE",
    "CALL:HT_TRENDLINE",
    "CALL:HT_TRENDMODE",
    "CALL:IMI",
    "CALL:LINEARREG",
    "CALL:LINEARREG_ANGLE",
    "CALL:LINEARREG_INTERCEPT",
    "CALL:MACDEXT",
    "CALL:MACDFIX",
    "CALL:MAVP",
    "CALL:MEDPRICE",
    "CALL:MIDPOINT",
    "CALL:MIDPRICE",
    "CALL:MINMAX",
    "CALL:MINMAXINDEX",
    "CALL:MINUS_DM",
    "CALL:PLUS_DM",
    "CALL:PERCENTILE",
    "CALL:PPO",
    "CALL:SAREXT",
    "CALL:STOCH",
    "CALL:STOCHRSI",
    "CALL:T3",
    "CALL:TEMA",
    "CALL:TSF",
    "CALL:TYPPRICE",
    "CALL:ULTOSC",
    "CALL:WCLPRICE",
];

/// Execute one TA-Lib formula function through the plan-owned buffers.
///
/// This fallback deliberately allocates only at the boundary: canonical legacy
/// functions accept `Array1<f64>` arguments and the hot-plan ABI owns `Vec<f64>`
/// buffers. The function table itself is cached, so subsequent calls avoid its
/// construction cost. Families with a proven in-place kernel should stay on
/// their specialized dispatch branch above.
pub(crate) fn dispatch_formula_bridge_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    name: &str,
) -> Result<(), KernelDispatchError> {
    let output_slot = call.output.0;
    let len = match buffers.get(output_slot) {
        Some(output) if !output.is_empty() => output.len(),
        _ => {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ))
        }
    };
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

    let args: Vec<Array1<f64>> = call
        .inputs
        .iter()
        .map(|slot| Array1::from(buffers[slot.0].clone()))
        .collect();
    let context_series = |index: usize| {
        args.get(index)
            .cloned()
            .unwrap_or_else(|| Array1::zeros(len))
    };
    let ctx = crate::formula::types::FormulaContext::new(
        context_series(0),
        context_series(1),
        context_series(2),
        context_series(3),
        context_series(4),
        None,
    );
    let Some(kernel) = crate::formula::functions::lookup_builtin_function(name) else {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_UNSUPPORTED_KERNEL,
        ));
    };
    match kernel(&ctx, &args) {
        Ok(series) if series.len() == len => {
            buffers[output_slot].copy_from_slice(series.as_slice().unwrap());
        }
        Ok(_) | Err(_) => {
            buffers[output_slot].fill(f64::NAN);
        }
    }
    Ok(())
}

/// Build the delegation context the `canonical_*` bridges are called with.
///
/// Every function behind `TALIB_081_KERNELS` and `SCREENING_DELEGATES` takes a
/// `&FormulaContext` but reads **only** `data_len` from it: the series arrive
/// through `args`, and the host-context channels (`chip`, `money_flow`,
/// `string_table`, ...) travel on `FormulaKernelDispatcher`'s own `HostContext`,
/// not on this one. Building the context at full length therefore allocated and
/// zero-filled five `len`-sized arrays per `CALL:` node for nothing — at 5000
/// bars that is 200 KB of memory traffic per call, which dominated the plan path
/// on call-heavy formulas (measured: `SMI`/`SMI_SIGNAL` at 4.36x the tree path).
/// `data_len` is a public field, so the length is set directly and the series are
/// left empty.
///
/// This is safe by evidence, not by assumption: all 57 functions referenced by
/// those two tables were audited for `ctx.<series>` reads and none exist.
/// `formula_plan_differential.rs` remains the independent safety net — if a
/// future delegate does read a context series, its plan-path output stops
/// matching the tree path and that gate fails.
///
/// Not used by [`dispatch_formula_bridge_call`], which deliberately maps the
/// first five arguments onto real OHLCV series because the builtin registry it
/// looks up contains functions that *do* read them.
pub(crate) fn bridge_context(len: usize) -> crate::formula::types::FormulaContext {
    let mut ctx = crate::formula::types::FormulaContext::new(
        Array1::zeros(0),
        Array1::zeros(0),
        Array1::zeros(0),
        Array1::zeros(0),
        Array1::zeros(0),
        None,
    );
    ctx.data_len = len;
    ctx
}
