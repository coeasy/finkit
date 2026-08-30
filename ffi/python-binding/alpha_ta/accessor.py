"""Pandas DataFrame accessor for AlphaTA technical analysis.

Usage::

    import pandas as pd
    import alpha_ta

    df = pd.read_csv("ohlcv.csv")
    rsi = df.ta.rsi(14)
    sma = df.ta.sma(20)
    result = df.ta.strategy([("sma", [14]), ("ema", [14]), ("rsi", [14])])
"""

from __future__ import annotations

from typing import Any, List, Optional, Sequence, Tuple, Union

import numpy as np

_REGISTERED = False


class TaAccessor:
    """Pandas DataFrame accessor providing ``df.ta.<indicator>(...)`` API."""

    _OHLCV_INDICATORS = {
        "atr", "adx", "stoch", "cci", "willr", "mfi", "obv", "bbands",
        "bollingerbands", "macd", "supertrend", "ichimoku", "vwap",
        "donchian", "elder_ray", "aroon", "dx", "plus_di", "minus_di",
        "natr", "trange", "bop", "ad", "adosc", "sar", "vortex",
        "heikin_ashi", "williams_alligator",
    }

    def __init__(self, pandas_obj):  # type: ignore[no-untyped-def]
        self._obj = pandas_obj

    @classmethod
    def _register(cls) -> None:
        global _REGISTERED
        if _REGISTERED:
            return
        try:
            import pandas as pd
            pd.api.extensions.register_dataframe_accessor("ta")(cls)
            _REGISTERED = True
        except Exception:
            pass

    def _get_col(self, column: Optional[str], default: str) -> np.ndarray:
        col = column or default
        return self._obj[col].values.astype(np.float64)

    def _call_indicator(self, name: str, *args: Any, **kwargs: Any) -> Any:
        try:
            import alpha_ta as _ta
        except ImportError:
            from alpha_ta import alpha_ta as _ta

        fn = getattr(_ta, name, None)
        if fn is None:
            raise AttributeError(f"alpha_ta has no indicator '{name}'")
        return fn(*args, **kwargs)

    def sma(self, period: int = 14, *, column: Optional[str] = None) -> Any:
        import pandas as pd
        close = self._get_col(column, "close")
        result = self._call_indicator("sma", close, timeperiod=period)
        return pd.Series(result, index=self._obj.index, name=f"sma_{period}")

    def ema(self, period: int = 14, *, column: Optional[str] = None) -> Any:
        import pandas as pd
        close = self._get_col(column, "close")
        result = self._call_indicator("ema", close, timeperiod=period)
        return pd.Series(result, index=self._obj.index, name=f"ema_{period}")

    def rsi(self, period: int = 14, *, column: Optional[str] = None) -> Any:
        import pandas as pd
        close = self._get_col(column, "close")
        result = self._call_indicator("rsi", close, timeperiod=period)
        return pd.Series(result, index=self._obj.index, name=f"rsi_{period}")

    def macd(
        self,
        fast: int = 12,
        slow: int = 26,
        signal: int = 9,
        *,
        column: Optional[str] = None,
    ) -> Any:
        import pandas as pd
        close = self._get_col(column, "close")
        result = self._call_indicator(
            "macd", close, fastperiod=fast, slowperiod=slow, signalperiod=signal
        )
        return pd.DataFrame(
            {
                "macd": result.macd if hasattr(result, "macd") else result["macd"],
                "signal": result.signal if hasattr(result, "signal") else result["signal"],
                "hist": result.hist if hasattr(result, "hist") else result["hist"],
            },
            index=self._obj.index,
        )

    def atr(self, period: int = 14) -> Any:
        import pandas as pd
        high = self._get_col(None, "high")
        low = self._get_col(None, "low")
        close = self._get_col(None, "close")
        result = self._call_indicator("atr", high, low, close, timeperiod=period)
        return pd.Series(result, index=self._obj.index, name=f"atr_{period}")

    def strategy(
        self,
        requests: Sequence[Tuple[str, List[int]]],
    ) -> Any:
        """Batch-compute multiple indicators and merge into a DataFrame."""
        import pandas as pd

        result_df = self._obj.copy()
        for name, params in requests:
            try:
                if name in self._OHLCV_INDICATORS:
                    fn = getattr(self, name, None)
                    if fn:
                        col = fn(*params)
                    else:
                        continue
                else:
                    fn = getattr(self, name, None)
                    if fn:
                        col = fn(*params)
                    else:
                        continue

                if isinstance(col, pd.Series):
                    col_name = f"{name}_{params[0]}" if params else name
                    result_df[col_name] = col.values
                elif isinstance(col, pd.DataFrame):
                    for c in col.columns:
                        result_df[f"{name}_{c}"] = col[c].values
            except Exception:
                continue

        return result_df


TaAccessor._register()
