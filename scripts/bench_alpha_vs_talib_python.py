#!/usr/bin/env python3
# ----------------------------------------------------------------------------
# AlphaTA vs TA-Lib — Full 158-function Python-level performance comparison.
#
# For each TA-Lib function:
#   1. Generate fixed random OHLCV data (seeded for reproducibility).
#   2. Call talib.<FUNC> (time it).
#   3. Call alpha_ta.<func> (time it).
#   4. Compare outputs (max abs diff ≤ 1e-6).
#   5. Calculate speedup ratio.
#
# Outputs:
#   dist/bench/python_comparison.json       — machine-readable
#   dist/bench/python_comparison.md         — Markdown report
#   dist/bench/python_comparison_summary.md — summary table
#
# Usage:
#   python scripts/bench_alpha_vs_talib_python.py --scale 10K
#   python scripts/bench_alpha_vs_talib_python.py --scale 100K --output dist/bench/
#
# Exit codes:
#   0  all functions compared successfully
#   1  at least one function had precision mismatch (> 1e-6)
#   2  missing dependency (alpha_ta / talib not installed)
# ----------------------------------------------------------------------------
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from datetime import datetime, timezone

import numpy as np

# ---- import guards --------------------------------------------------------
try:
    import alpha_ta
except ImportError:
    print("[bench] alpha_ta not installed; build and install the wheel first",
          file=sys.stderr)
    sys.exit(2)

try:
    import talib
except ImportError:
    print("[bench] talib not installed; pip install TA-Lib", file=sys.stderr)
    sys.exit(2)


# ----------------------------------------------------------------------------
# Function registry — 158 TA-Lib functions mapped to alpha_ta equivalents.
#
# Each entry:
#   talib:   TA-Lib function name (uppercase)
#   alpha:   alpha_ta function name (lowercase) — None if no equivalent
#   category: indicator category
#   inputs:  list of input array keys from the OHLCV tuple
#   params:  dict of keyword parameters
#   returns: expected number of output arrays (1, 2, or 3)
# ----------------------------------------------------------------------------
FUNCTIONS = [
    # ========================================================================
    # Overlap Studies (18)
    # ========================================================================
    {"talib": "SMA",        "alpha": "sma",        "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "EMA",        "alpha": "ema",        "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "WMA",        "alpha": "wma",        "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "DEMA",       "alpha": "dema",       "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "TEMA",       "alpha": "tema",       "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "TRIMA",      "alpha": "trima",      "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "KAMA",       "alpha": "kama",       "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "MAVP",       "alpha": None,          "category": "Overlap",  "inputs": ["close"],   "params": {},                          "returns": 1},  # MAVP needs variable periods
    {"talib": "T3",         "alpha": "t3",         "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 5, "vfactor": 0.7}, "returns": 1},
    {"talib": "MA",         "alpha": "sma",        "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},  # MA default = SMA
    {"talib": "BBANDS",     "alpha": "bollinger_bands", "category": "Overlap", "inputs": ["close"], "params": {"timeperiod": 20, "nbdevup": 2.0, "nbdevdn": 2.0}, "returns": 3},
    {"talib": "SAR",        "alpha": "sar",        "category": "Overlap",  "inputs": ["high", "low"], "params": {"acceleration": 0.02, "maximum": 0.2}, "returns": 1},
    {"talib": "SAREXT",     "alpha": "sar",        "category": "Overlap",  "inputs": ["high", "low"], "params": {"acceleration": 0.02, "maximum": 0.2}, "returns": 1},  # SAREXT with default params
    {"talib": "MIDPOINT",   "alpha": "midpoint",   "category": "Overlap",  "inputs": ["close"],   "params": {"timeperiod": 14},         "returns": 1},
    {"talib": "MIDPRICE",   "alpha": "midprice",   "category": "Overlap",  "inputs": ["high", "low"], "params": {"timeperiod": 14},      "returns": 1},
    {"talib": "HT_TRENDLINE","alpha": "ht_trendline","category": "Overlap", "inputs": ["close"],   "params": {},                          "returns": 1},
    {"talib": "MAMA",       "alpha": "mama",       "category": "Overlap",  "inputs": ["close"],   "params": {"fastlimit": 0.5, "slowlimit": 0.05}, "returns": 2},
    {"talib": "FRAMA",      "alpha": None,          "category": "Overlap",  "inputs": ["close"],   "params": {},                          "returns": 1},  # FRAMA not in alpha_ta

    # ========================================================================
    # Momentum (30)
    # ========================================================================
    {"talib": "RSI",        "alpha": "rsi",        "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 14},         "returns": 1},
    {"talib": "MACD",       "alpha": "macd",       "category": "Momentum", "inputs": ["close"],   "params": {"fastperiod": 12, "slowperiod": 26, "signalperiod": 9}, "returns": 3},
    {"talib": "MACDEXT",    "alpha": "macd",       "category": "Momentum", "inputs": ["close"],   "params": {"fastperiod": 12, "slowperiod": 26, "signalperiod": 9}, "returns": 3},
    {"talib": "MACDFIX",    "alpha": "macd",       "category": "Momentum", "inputs": ["close"],   "params": {"fastperiod": 12, "slowperiod": 26, "signalperiod": 9}, "returns": 3},
    {"talib": "STOCH",      "alpha": "stoch",      "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"fastk_period": 5, "slowk_period": 3, "slowd_period": 3}, "returns": 2},
    {"talib": "STOCHF",     "alpha": "stochf",     "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"fastk_period": 5, "fastd_period": 3}, "returns": 2},
    {"talib": "STOCHRSI",   "alpha": "stochrsi",   "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 14, "fastk_period": 5, "fastd_period": 3}, "returns": 2},
    {"talib": "WILLR",      "alpha": "willr",      "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "ADX",        "alpha": "adx",        "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "ADXR",       "alpha": "adxr",       "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "APO",        "alpha": "apo",        "category": "Momentum", "inputs": ["close"],   "params": {"fastperiod": 12, "slowperiod": 26}, "returns": 1},
    {"talib": "PPO",        "alpha": "ppo",        "category": "Momentum", "inputs": ["close"],   "params": {"fastperiod": 12, "slowperiod": 26}, "returns": 1},
    {"talib": "AROON",      "alpha": "aroon",      "category": "Momentum", "inputs": ["high", "low"], "params": {"timeperiod": 14},      "returns": 2},
    {"talib": "AROONOSC",   "alpha": "aroonosc",   "category": "Momentum", "inputs": ["high", "low"], "params": {"timeperiod": 14},      "returns": 1},
    {"talib": "BOP",        "alpha": "bop",        "category": "Momentum", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CCI",        "alpha": "cci",        "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "CMO",        "alpha": "cmo",        "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 14},         "returns": 1},
    {"talib": "DX",         "alpha": "dx",         "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "MFI",        "alpha": "mfi",        "category": "Momentum", "inputs": ["high", "low", "close", "volume"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "MINUS_DI",   "alpha": "minus_di",   "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "MINUS_DM",   "alpha": "minus_dm",   "category": "Momentum", "inputs": ["high", "low"], "params": {"timeperiod": 14},      "returns": 1},
    {"talib": "MOM",        "alpha": "mom",        "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 10},         "returns": 1},
    {"talib": "PLUS_DI",    "alpha": "plus_di",    "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "PLUS_DM",    "alpha": "plus_dm",    "category": "Momentum", "inputs": ["high", "low"], "params": {"timeperiod": 14},      "returns": 1},
    {"talib": "ROC",        "alpha": "roc",        "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 10},         "returns": 1},
    {"talib": "ROCP",       "alpha": "rocp",       "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 10},         "returns": 1},
    {"talib": "ROCR",       "alpha": "rocr",       "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 10},         "returns": 1},
    {"talib": "ROCR100",    "alpha": "rocr100",    "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 10},         "returns": 1},
    {"talib": "TRIX",       "alpha": "trix",       "category": "Momentum", "inputs": ["close"],   "params": {"timeperiod": 30},         "returns": 1},
    {"talib": "ULTOSC",     "alpha": "ultosc",     "category": "Momentum", "inputs": ["high", "low", "close"], "params": {"timeperiod1": 7, "timeperiod2": 14, "timeperiod3": 28}, "returns": 1},

    # ========================================================================
    # Volume (3)
    # ========================================================================
    {"talib": "AD",         "alpha": "ad",         "category": "Volume",   "inputs": ["high", "low", "close", "volume"], "params": {}, "returns": 1},
    {"talib": "ADOSC",      "alpha": "adosc",      "category": "Volume",   "inputs": ["high", "low", "close", "volume"], "params": {"fastperiod": 3, "slowperiod": 10}, "returns": 1},
    {"talib": "OBV",        "alpha": "obv",        "category": "Volume",   "inputs": ["close", "volume"], "params": {},                "returns": 1},

    # ========================================================================
    # Volatility (3)
    # ========================================================================
    {"talib": "ATR",        "alpha": "atr",        "category": "Volatility","inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "NATR",       "alpha": "natr",       "category": "Volatility","inputs": ["high", "low", "close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "TRANGE",     "alpha": "trange",     "category": "Volatility","inputs": ["high", "low", "close"], "params": {},             "returns": 1},

    # ========================================================================
    # Price Transform (4)
    # ========================================================================
    {"talib": "AVGPRICE",   "alpha": "avgprice",   "category": "Price",    "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "MEDPRICE",   "alpha": "medprice",   "category": "Price",    "inputs": ["high", "low"], "params": {},                     "returns": 1},
    {"talib": "TYPPRICE",   "alpha": "typprice",   "category": "Price",    "inputs": ["high", "low", "close"], "params": {},             "returns": 1},
    {"talib": "WCLPRICE",   "alpha": "wclprice",   "category": "Price",    "inputs": ["high", "low", "close"], "params": {},             "returns": 1},

    # ========================================================================
    # Cycle (6)
    # ========================================================================
    {"talib": "HT_DCPERIOD",  "alpha": "ht_dcperiod",  "category": "Cycle", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "HT_DCPHASE",   "alpha": "ht_dcphase",   "category": "Cycle", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "HT_PHASOR",    "alpha": "ht_phasor",    "category": "Cycle", "inputs": ["close"], "params": {}, "returns": 2},
    {"talib": "HT_SINE",      "alpha": "ht_sine",      "category": "Cycle", "inputs": ["close"], "params": {}, "returns": 2},
    {"talib": "HT_TRENDMODE", "alpha": "ht_trendmode", "category": "Cycle", "inputs": ["close"], "params": {}, "returns": 1},

    # ========================================================================
    # Statistics (9)
    # ========================================================================
    {"talib": "BETA",      "alpha": "beta",      "category": "Statistics", "inputs": ["high", "low"], "params": {"timeperiod": 5},  "returns": 1},
    {"talib": "CORREL",    "alpha": "correl",    "category": "Statistics", "inputs": ["high", "low"], "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "LINEARREG", "alpha": "linearreg", "category": "Statistics", "inputs": ["close"],   "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "LINEARREG_ANGLE",     "alpha": "linearreg_angle",     "category": "Statistics", "inputs": ["close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "LINEARREG_INTERCEPT", "alpha": "linearreg_intercept", "category": "Statistics", "inputs": ["close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "LINEARREG_SLOPE",     "alpha": "linearreg_slope",     "category": "Statistics", "inputs": ["close"], "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "STDDEV",    "alpha": "stddev",    "category": "Statistics", "inputs": ["close"],   "params": {"timeperiod": 5, "nbdev": 1.0}, "returns": 1},
    {"talib": "TSF",       "alpha": "tsf",       "category": "Statistics", "inputs": ["close"],   "params": {"timeperiod": 14}, "returns": 1},
    {"talib": "VAR",       "alpha": "var",       "category": "Statistics", "inputs": ["close"],   "params": {"timeperiod": 5, "nbdev": 1.0}, "returns": 1},

    # ========================================================================
    # Math Transform (15)
    # ========================================================================
    {"talib": "ACOS",  "alpha": "acos",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "ASIN",  "alpha": "asin",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "ATAN",  "alpha": "atan",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "CEIL",  "alpha": "ceil",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "COS",   "alpha": "cos",   "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "COSH",  "alpha": "cosh",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "EXP",   "alpha": "exp",   "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "FLOOR", "alpha": "floor", "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "LN",    "alpha": "ln",    "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "LOG10", "alpha": "log10", "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "SIN",   "alpha": "sin",   "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "SINH",  "alpha": "sinh",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "SQRT",  "alpha": "sqrt",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "TAN",   "alpha": "tan",   "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},
    {"talib": "TANH",  "alpha": "tanh",  "category": "MathTransform", "inputs": ["close"], "params": {}, "returns": 1},

    # ========================================================================
    # Math Operators (12 + 2)
    # ========================================================================
    {"talib": "ADD",          "alpha": "add",          "category": "MathOperator", "inputs": ["high", "low"], "params": {},                "returns": 1},
    {"talib": "DIV",          "alpha": "div",          "category": "MathOperator", "inputs": ["high", "low"], "params": {},                "returns": 1},
    {"talib": "MAX",          "alpha": "max",          "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "MAXINDEX",     "alpha": "maxindex",     "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "MIN",          "alpha": "min",          "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "MININDEX",     "alpha": "minindex",     "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "MINMAX",       "alpha": "minmax",       "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 2},
    {"talib": "MINMAXINDEX",  "alpha": "minmaxindex",  "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 2},
    {"talib": "MULT",         "alpha": "mult",         "category": "MathOperator", "inputs": ["high", "low"], "params": {},                "returns": 1},
    {"talib": "SUB",          "alpha": "sub",          "category": "MathOperator", "inputs": ["high", "low"], "params": {},                "returns": 1},
    {"talib": "SUM",          "alpha": "sum",          "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "PERCENTRANK",  "alpha": "percentrank",  "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "SKEWNESS",     "alpha": "skewness",     "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},
    {"talib": "KURTOSIS",     "alpha": "kurtosis",     "category": "MathOperator", "inputs": ["close"],      "params": {"timeperiod": 30}, "returns": 1},

    # ========================================================================
    # Pattern Recognition (61) — CDL_* functions
    # ========================================================================
    {"talib": "CDL2CROWS",           "alpha": "cdl2crows",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3BLACKCROWS",      "alpha": "cdl3blackcrows",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3INSIDE",          "alpha": "cdl3inside",          "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3LINESTRIKE",      "alpha": "cdl3linestrike",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3OUTSIDE",         "alpha": "cdl3outside",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3STARSINSOUTH",    "alpha": "cdl3starsinsouth",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDL3WHITESOLDIERS",   "alpha": "cdl3whitesoldiers",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLABANDONEDBABY",    "alpha": "cdlabandonedbaby",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.3}, "returns": 1},
    {"talib": "CDLADVANCEBLOCK",     "alpha": "cdladvanceblock",     "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLBELTHOLD",         "alpha": "cdlbelthold",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLBREAKAWAY",        "alpha": "cdlbreakaway",        "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLCLOSINGMARUBOZU",  "alpha": "cdlclosingmarubozu",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLCONCEALBABYSWALL", "alpha": "cdlconcealbabyswall", "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLCOUNTERATTACK",    "alpha": "cdlcounterattack",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLDARKCLOUDCOVER",   "alpha": "cdldarkcloudcover",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.5}, "returns": 1},
    {"talib": "CDLDOJI",             "alpha": "cdldoji",             "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLDOJISTAR",         "alpha": "cdldojistar",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLDRAGONFLYDOJI",    "alpha": "cdldragonflydoji",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLENGULFING",        "alpha": "cdlengulfing",        "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLEVENINGDOJISTAR",  "alpha": "cdleveningdojistar",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.3}, "returns": 1},
    {"talib": "CDLEVENINGSTAR",      "alpha": "cdleveningstar",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.3}, "returns": 1},
    {"talib": "CDLGAPSIDESIDEWHITE", "alpha": "cdlgapsidesidewhite", "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLGRAVESTONEDOJI",   "alpha": "cdlgravestonedoji",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHAMMER",           "alpha": "cdlhammer",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHANGINGMAN",       "alpha": "cdlhangingman",       "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHARAMI",           "alpha": "cdlharami",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHARAMICROSS",      "alpha": "cdlharamicross",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHIGHWAVE",         "alpha": "cdlhighwave",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHIKKAKE",          "alpha": "cdlhikkake",          "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHIKKAKEMOD",       "alpha": "cdlhikkakemod",       "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLHOMINGPIGEON",     "alpha": "cdlhomingsoldier",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLIDENTICAL3CROWS",  "alpha": "cdlidentical3crows",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLINNECK",           "alpha": "cdlinneck",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLINVERTEDHAMMER",   "alpha": "cdlinvertedhammer",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLKICKING",          "alpha": "cdlkicking",          "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLKICKINGBYLENGTH",  "alpha": "cdlkickingbylength",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLLADDERBOTTOM",     "alpha": "cdlladderbottom",     "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLLONGLEGGEDDOJI",   "alpha": "cdllongleggeddoji",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLLONGLINE",         "alpha": "cdllongline",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLMARUBOZU",         "alpha": "cdlmarubozu",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLMATCHINGLOW",      "alpha": "cdlmatchinglow",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLMATHOLD",          "alpha": "cdlmathold",          "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.5}, "returns": 1},
    {"talib": "CDLMORNINGDOJISTAR",  "alpha": "cdlmorningdojistar",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.3}, "returns": 1},
    {"talib": "CDLMORNINGSTAR",      "alpha": "cdlmorningstar",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {"penetration": 0.3}, "returns": 1},
    {"talib": "CDLONNECK",           "alpha": "cdlonneck",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLPIERCING",         "alpha": "cdlpiercing",         "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLRICKSHAWMAN",      "alpha": "cdlrickshawman",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLRISEFALL3METHODS", "alpha": "cdlrisefall3methods", "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSEPARATINGLINES",  "alpha": "cdlseparatinglines",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSHOOTINGSTAR",     "alpha": "cdlshootingstar",     "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSHORTLINE",        "alpha": "cdlshortline",        "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSPINNINGTOP",      "alpha": "cdlspinningtop",      "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSTALLEDPATTERN",   "alpha": "cdlstalledpattern",   "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLSTICKSANDWICH",    "alpha": "cdlsticksandwich",    "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLTAKURI",           "alpha": "cdltakuri",           "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLTASUKIGAP",        "alpha": "cdltasukigap",        "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLTHRUSTING",        "alpha": "cdlthrusting",        "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLTRISTAR",          "alpha": "cdltristar",          "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLUNIQUE3RIVER",     "alpha": "cdlunique3river",     "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLUPSIDEGAP2CROWS",  "alpha": "cdlupsidegap2crows",  "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
    {"talib": "CDLXSIDEGAP3METHODS", "alpha": "cdlxsidegap3methods", "category": "Pattern", "inputs": ["open", "high", "low", "close"], "params": {}, "returns": 1},
]


# ----------------------------------------------------------------------------
# Input data generation
# ----------------------------------------------------------------------------
def gen_inputs(n: int = 10_000, seed: int = 42) -> dict:
    """Generate fixed random OHLCV data for reproducible benchmarks."""
    rng = np.random.default_rng(seed)
    close = np.cumsum(rng.standard_normal(n).astype(np.float64)) + 100.0
    # Ensure positive prices
    close = np.maximum(close, 1.0)
    high = close + rng.uniform(0.1, 1.5, n).astype(np.float64)
    low = np.maximum(close - rng.uniform(0.1, 1.5, n).astype(np.float64), 0.5)
    open_ = close + rng.standard_normal(n).astype(np.float64) * 0.2
    volume = rng.integers(1_000, 1_000_000, n).astype(np.float64)
    return {
        "open": open_,
        "high": high,
        "low": low,
        "close": close,
        "volume": volume,
    }


# ----------------------------------------------------------------------------
# Timing utility
# ----------------------------------------------------------------------------
def time_call(func, *args, repeat: int = 5, **kwargs) -> float:
    """Time a function call, returning the median time in seconds."""
    times = []
    for _ in range(repeat):
        t0 = time.perf_counter()
        _ = func(*args, **kwargs)
        t1 = time.perf_counter()
        times.append(t1 - t0)
    return float(np.median(times))


def to_list(result):
    """Normalize result to a list of numpy arrays."""
    if isinstance(result, tuple):
        return [np.asarray(r, dtype=np.float64) for r in result]
    return [np.asarray(result, dtype=np.float64)]


def compare_arrays(alpha_arrs: list, talib_arrs: list) -> dict:
    """Compare two lists of arrays, return max abs diff and match ratio."""
    max_abs = 0.0
    valid_count = 0
    total_count = 0
    for a, t in zip(alpha_arrs, talib_arrs):
        if a.shape != t.shape:
            return {"max_abs": float("inf"), "match_ratio": 0.0, "shape_mismatch": True}
        valid = np.isfinite(t) & np.isfinite(a)
        if valid.any():
            diff = np.abs(a[valid] - t[valid])
            max_abs = max(max_abs, float(diff.max()) if diff.size > 0 else 0.0)
        valid_count += int(valid.sum())
        total_count += a.size
    match_ratio = valid_count / total_count if total_count > 0 else 0.0
    return {"max_abs": max_abs, "match_ratio": match_ratio, "shape_mismatch": False}


# ----------------------------------------------------------------------------
# Main benchmark logic
# ----------------------------------------------------------------------------
def run_benchmark(scale: int, output_dir: str, repeat: int = 5):
    """Run the full 158-function comparison benchmark."""
    data = gen_inputs(scale)
    results = []
    skipped = []
    errors = []

    total = len(FUNCTIONS)
    print(f"\n[bench] Comparing {total} functions at scale={scale} bars")
    print(f"[bench] Data: {scale} bars OHLCV (seed=42)")
    print(f"[bench] Repeat: {repeat} iterations per function (median)\n")

    for i, spec in enumerate(FUNCTIONS, 1):
        talib_name = spec["talib"]
        alpha_name = spec["alpha"]
        category = spec["category"]

        # Get TA-Lib function
        talib_func = getattr(talib, talib_name, None)
        if talib_func is None:
            skipped.append({"name": talib_name, "reason": "not in talib"})
            continue

        # Skip if no alpha_ta equivalent
        if alpha_name is None:
            skipped.append({"name": talib_name, "reason": "no alpha_ta equivalent"})
            continue

        alpha_func = getattr(alpha_ta, alpha_name, None)
        if alpha_func is None:
            skipped.append({"name": talib_name, "reason": f"alpha_ta.{alpha_name} not found"})
            continue

        # Prepare inputs
        inputs = [data[k] for k in spec["inputs"]]
        params = spec["params"]

        # Time TA-Lib
        try:
            talib_time = time_call(talib_func, *inputs, repeat=repeat, **params)
            talib_result = talib_func(*inputs, **params)
            talib_arrs = to_list(talib_result)
        except Exception as e:
            errors.append({"name": talib_name, "side": "talib", "error": str(e)})
            continue

        # Time alpha_ta
        try:
            alpha_time = time_call(alpha_func, *inputs, repeat=repeat, **params)
            alpha_result = alpha_func(*inputs, **params)
            alpha_arrs = to_list(alpha_result)
        except Exception as e:
            errors.append({"name": talib_name, "side": "alpha_ta", "error": str(e)})
            continue

        # Compare outputs
        cmp = compare_arrays(alpha_arrs, talib_arrs)
        speedup = talib_time / alpha_time if alpha_time > 0 else float("inf")

        entry = {
            "talib_name": talib_name,
            "alpha_name": alpha_name,
            "category": category,
            "inputs": spec["inputs"],
            "params": params,
            "talib_time_ms": talib_time * 1000,
            "alpha_time_ms": alpha_time * 1000,
            "speedup": speedup,
            "max_abs_diff": cmp["max_abs"],
            "match_ratio": cmp["match_ratio"],
            "shape_mismatch": cmp["shape_mismatch"],
            "precision_ok": cmp["max_abs"] < 1e-6,
        }
        results.append(entry)

        status = "✓" if entry["precision_ok"] else "⚠"
        print(f"  [{i:3d}/{total}] {status} {talib_name:25s} | "
              f"talib={talib_time*1000:8.3f}ms  alpha={alpha_time*1000:8.3f}ms  "
              f"speedup={speedup:6.2f}x  diff={cmp['max_abs']:.2e}")

    # Summary
    compared = len(results)
    precision_pass = sum(1 for r in results if r["precision_ok"])
    precision_fail = compared - precision_pass
    avg_speedup = np.mean([r["speedup"] for r in results]) if results else 0
    max_speedup = max((r["speedup"] for r in results), default=0)
    min_speedup = min((r["speedup"] for r in results), default=0)

    summary = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "scale": scale,
        "repeat": repeat,
        "total_functions": total,
        "compared": compared,
        "skipped": len(skipped),
        "errors": len(errors),
        "precision_pass": precision_pass,
        "precision_fail": precision_fail,
        "avg_speedup": float(avg_speedup),
        "max_speedup": float(max_speedup),
        "min_speedup": float(min_speedup),
    }

    print(f"\n[bench] Summary:")
    print(f"  Compared:     {compared}/{total}")
    print(f"  Skipped:      {len(skipped)}")
    print(f"  Errors:       {len(errors)}")
    print(f"  Precision OK: {precision_pass}/{compared}")
    print(f"  Avg speedup:  {avg_speedup:.2f}x")
    print(f"  Max speedup:  {max_speedup:.2f}x")
    print(f"  Min speedup:  {min_speedup:.2f}x")

    # Write outputs
    os.makedirs(output_dir, exist_ok=True)

    # JSON
    json_path = os.path.join(output_dir, "python_comparison.json")
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump({"summary": summary, "results": results, "skipped": skipped, "errors": errors}, f, indent=2, ensure_ascii=False)
    print(f"\n[bench] JSON:       {json_path}")

    # Markdown report
    md_path = os.path.join(output_dir, "python_comparison.md")
    write_markdown_report(md_path, summary, results, skipped, errors)
    print(f"[bench] Markdown:   {md_path}")

    # Summary table
    summary_path = os.path.join(output_dir, "python_comparison_summary.md")
    write_summary_table(summary_path, summary, results)
    print(f"[bench] Summary:    {summary_path}")

    return 0 if precision_fail == 0 else 1


def write_markdown_report(path, summary, results, skipped, errors):
    """Write the full Markdown report."""
    with open(path, "w", encoding="utf-8") as f:
        f.write("# AlphaTA vs TA-Lib: Python-Level Performance Comparison\n\n")
        f.write(f"**Date:** {summary['timestamp']}\n\n")
        f.write(f"**Scale:** {summary['scale']:,} bars OHLCV (seed=42)\n\n")
        f.write(f"**Repeat:** {summary['repeat']} iterations (median)\n\n")
        f.write(f"**Functions compared:** {summary['compared']}/{summary['total_functions']}\n\n")
        f.write("---\n\n")

        # Summary stats
        f.write("## Summary\n\n")
        f.write(f"| Metric | Value |\n|--------|-------|\n")
        f.write(f"| Total functions | {summary['total_functions']} |\n")
        f.write(f"| Compared | {summary['compared']} |\n")
        f.write(f"| Skipped | {summary['skipped']} |\n")
        f.write(f"| Errors | {summary['errors']} |\n")
        f.write(f"| Precision pass (≤1e-6) | {summary['precision_pass']} |\n")
        f.write(f"| Precision fail | {summary['precision_fail']} |\n")
        f.write(f"| Average speedup | {summary['avg_speedup']:.2f}x |\n")
        f.write(f"| Max speedup | {summary['max_speedup']:.2f}x |\n")
        f.write(f"| Min speedup | {summary['min_speedup']:.2f}x |\n\n")

        # Per-category breakdown
        categories = {}
        for r in results:
            cat = r["category"]
            if cat not in categories:
                categories[cat] = []
            categories[cat].append(r)

        f.write("## Per-Category Breakdown\n\n")
        f.write("| Category | Functions | Avg Speedup | Max Speedup | Precision Pass |\n")
        f.write("|----------|-----------|-------------|-------------|----------------|\n")
        for cat in sorted(categories.keys()):
            cat_results = categories[cat]
            avg_sp = np.mean([r["speedup"] for r in cat_results])
            max_sp = max(r["speedup"] for r in cat_results)
            pass_count = sum(1 for r in cat_results if r["precision_ok"])
            f.write(f"| {cat} | {len(cat_results)} | {avg_sp:.2f}x | {max_sp:.2f}x | {pass_count}/{len(cat_results)} |\n")

        # Detailed results
        f.write("\n## Detailed Results\n\n")
        for cat in sorted(categories.keys()):
            cat_results = categories[cat]
            f.write(f"### {cat} ({len(cat_results)} functions)\n\n")
            f.write("| TA-Lib | AlphaTA | talib (ms) | alpha (ms) | Speedup | Max Diff | Status |\n")
            f.write("|--------|---------|------------|------------|---------|----------|--------|\n")
            for r in sorted(cat_results, key=lambda x: -x["speedup"]):
                status = "✅" if r["precision_ok"] else "⚠️"
                f.write(f"| {r['talib_name']} | {r['alpha_name']} | {r['talib_time_ms']:.3f} | "
                        f"{r['alpha_time_ms']:.3f} | {r['speedup']:.2f}x | "
                        f"{r['max_abs_diff']:.2e} | {status} |\n")
            f.write("\n")

        # Skipped
        if skipped:
            f.write("## Skipped Functions\n\n")
            f.write("| Function | Reason |\n|----------|--------|\n")
            for s in skipped:
                f.write(f"| {s['name']} | {s['reason']} |\n")
            f.write("\n")

        # Errors
        if errors:
            f.write("## Errors\n\n")
            f.write("| Function | Side | Error |\n|----------|------|-------|\n")
            for e in errors:
                f.write(f"| {e['name']} | {e['side']} | {e['error'][:100]} |\n")


def write_summary_table(path, summary, results):
    """Write a condensed summary table."""
    with open(path, "w", encoding="utf-8") as f:
        f.write("# AlphaTA vs TA-Lib: Performance Summary\n\n")
        f.write(f"**Scale:** {summary['scale']:,} bars | **Repeat:** {summary['repeat']}x | "
                f"**Date:** {summary['timestamp'][:10]}\n\n")

        # Top 10 fastest
        f.write("## Top 10 Fastest (AlphaTA vs TA-Lib)\n\n")
        f.write("| Function | Category | talib (ms) | alpha (ms) | Speedup |\n")
        f.write("|----------|----------|------------|------------|---------|\n")
        top10 = sorted(results, key=lambda x: -x["speedup"])[:10]
        for r in top10:
            f.write(f"| {r['talib_name']} | {r['category']} | {r['talib_time_ms']:.3f} | "
                    f"{r['alpha_time_ms']:.3f} | **{r['speedup']:.2f}x** |\n")

        # Bottom 5
        f.write("\n## Bottom 5 (closest to TA-Lib speed)\n\n")
        f.write("| Function | Category | talib (ms) | alpha (ms) | Speedup |\n")
        f.write("|----------|----------|------------|------------|---------|\n")
        bottom5 = sorted(results, key=lambda x: x["speedup"])[:5]
        for r in bottom5:
            f.write(f"| {r['talib_name']} | {r['category']} | {r['talib_time_ms']:.3f} | "
                    f"{r['alpha_time_ms']:.3f} | **{r['speedup']:.2f}x** |\n")

        # Precision summary
        f.write(f"\n## Precision\n\n")
        f.write(f"- **Pass (≤1e-6):** {summary['precision_pass']}/{summary['compared']}\n")
        f.write(f"- **Fail:** {summary['precision_fail']}/{summary['compared']}\n")
        if summary["precision_fail"] > 0:
            fails = [r for r in results if not r["precision_ok"]]
            f.write("\n| Function | Max Diff |\n|----------|----------|\n")
            for r in fails:
                f.write(f"| {r['talib_name']} | {r['max_abs_diff']:.2e} |\n")


# ----------------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(
        description="AlphaTA vs TA-Lib: Full 158-function Python-level comparison")
    parser.add_argument("--scale", type=str, default="10K",
                        choices=["1K", "10K", "100K", "1M"],
                        help="Data size (number of bars)")
    parser.add_argument("--output", type=str, default="dist/bench",
                        help="Output directory for reports")
    parser.add_argument("--repeat", type=int, default=5,
                        help="Number of timing iterations per function (median)")
    args = parser.parse_args()

    scale_map = {"1K": 1_000, "10K": 10_000, "100K": 100_000, "1M": 1_000_000}
    scale = scale_map[args.scale]

    print(f"[bench] AlphaTA version: {getattr(alpha_ta, '__version__', 'unknown')}")
    print(f"[bench] TA-Lib version: {talib.__version__}")
    print(f"[bench] NumPy version: {np.__version__}")

    return run_benchmark(scale, args.output, args.repeat)


if __name__ == "__main__":
    sys.exit(main())
