"""Type stubs for finkit Python bindings.

This file provides type hints for the native Rust extension module.
Auto-generated from Rust FFI signatures.
"""

from typing import Any, Dict, List, Optional, Sequence, Tuple, Union
import numpy as np
from numpy.typing import NDArray

# Type aliases
ArrayLike = Union[NDArray[np.floating], List[float], Tuple[float, ...]]
Array1D = NDArray[np.floating]

def resolve_market_session(
    market: str,
    timestamp: int,
    timezone: Optional[str] = ...,
    holidays: Optional[List[str]] = ...,
    sessions: Optional[List[Tuple[int, int]]] = ...,
    special_sessions: Optional[List[Tuple[str, List[Tuple[int, int]]]]] = ...,
) -> Optional[Dict[str, Any]]:
    """Resolve a Unix timestamp using a built-in market calendar preset."""
    ...

def resolve_market_session_config(config_json: str, timestamp: int) -> Optional[Dict[str, Any]]:
    """Resolve a Unix timestamp from a versioned JSON exchange calendar."""
    ...

def resolve_market_session_csv(
    csv: str,
    market: str,
    timestamp: int,
    timezone: Optional[str] = ...,
) -> Optional[Dict[str, Any]]:
    """Resolve a Unix timestamp from an exchange-published annual CSV calendar."""
    ...

def compute_composite(
    close: ArrayLike,
    definitions: List[Tuple[str, str, List[str], List[float]]],
    outputs: Optional[List[str]] = ...,
    open: Optional[ArrayLike] = ...,
    high: Optional[ArrayLike] = ...,
    low: Optional[ArrayLike] = ...,
    volume: Optional[ArrayLike] = ...,
) -> Dict[str, Array1D]:
    """Evaluate a dependency-aware graph of custom composite indicators."""
    ...

# ============================================================================
# Overlap Studies
# ============================================================================

def sma(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Simple Moving Average."""
    ...

def ema(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Exponential Moving Average."""
    ...

def wma(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Weighted Moving Average."""
    ...

def dema(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Double Exponential Moving Average."""
    ...

def tema(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Triple Exponential Moving Average."""
    ...

def kama(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Kaufman Adaptive Moving Average."""
    ...

def mama(close: ArrayLike, *, fastlimit: float = 0.5, slowlimit: float = 0.05) -> Tuple[Array1D, Array1D]:
    """MESA Adaptive Moving Average. Returns (mama, fama)."""
    ...

def t3(close: ArrayLike, *, timeperiod: int = 5, vfactor: float = 0.7) -> Array1D:
    """Triple Exponential Moving Average (T3)."""
    ...

def bollinger_bands(close: ArrayLike, *, timeperiod: int = 20, nbdevup: float = 2.0, nbdevdn: float = 2.0, matype: int = 0) -> Tuple[Array1D, Array1D, Array1D]:
    """Bollinger Bands. Returns (upperband, middleband, lowerband)."""
    ...

def sar(high: ArrayLike, low: ArrayLike, *, acceleration: float = 0.02, maximum: float = 0.2) -> Array1D:
    """Parabolic SAR."""
    ...

def midpoint(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """MidPoint over period."""
    ...

def midprice(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Midprice over period."""
    ...

# ============================================================================
# Momentum Indicators
# ============================================================================

def rsi(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Relative Strength Index."""
    ...

def macd(close: ArrayLike, *, fastperiod: int = 12, slowperiod: int = 26, signalperiod: int = 9) -> Tuple[Array1D, Array1D, Array1D]:
    """MACD. Returns (macd, signal, hist)."""
    ...

def stoch(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, fastk_period: int = 5, slowk_period: int = 3, slowk_matype: int = 0, slowd_period: int = 3, slowd_matype: int = 0) -> Tuple[Array1D, Array1D]:
    """Stochastic. Returns (slowk, slowd)."""
    ...

def adx(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Average Directional Movement Index."""
    ...

def aroon(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 14) -> Tuple[Array1D, Array1D]:
    """Aroon. Returns (aroondown, aroonup)."""
    ...

def cci(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Commodity Channel Index."""
    ...

def mom(close: ArrayLike, *, timeperiod: int = 10) -> Array1D:
    """Momentum."""
    ...

def roc(close: ArrayLike, *, timeperiod: int = 10) -> Array1D:
    """Rate of change."""
    ...

def willr(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Williams' %R."""
    ...

def apo(close: ArrayLike, *, fastperiod: int = 12, slowperiod: int = 26, matype: int = 0) -> Array1D:
    """Absolute Price Oscillator."""
    ...

def bop(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Balance Of Power."""
    ...

def cmo(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Chande Momentum Oscillator."""
    ...

def dx(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Directional Movement Index."""
    ...

def mfi(high: ArrayLike, low: ArrayLike, close: ArrayLike, volume: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Money Flow Index."""
    ...

def minus_di(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Minus Directional Indicator."""
    ...

def minus_dm(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Minus Directional Movement."""
    ...

def plus_di(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Plus Directional Indicator."""
    ...

def plus_dm(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Plus Directional Movement."""
    ...

def trix(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """1-day Rate-Of-Change (ROC) of a Triple Smooth EMA."""
    ...

# ============================================================================
# Cycle Indicators
# ============================================================================

def ht_dcperiod(close: ArrayLike) -> Array1D:
    """Hilbert Transform - Dominant Cycle Period."""
    ...

def ht_dcphase(close: ArrayLike) -> Array1D:
    """Hilbert Transform - Dominant Cycle Phase."""
    ...

def ht_phasor(close: ArrayLike) -> Tuple[Array1D, Array1D]:
    """Hilbert Transform - Phasor Components. Returns (inphase, quadrature)."""
    ...

def ht_sine(close: ArrayLike) -> Tuple[Array1D, Array1D]:
    """Hilbert Transform - SineWave. Returns (sine, leadsine)."""
    ...

def ht_trendmode(close: ArrayLike) -> Array1D:
    """Hilbert Transform - Trend vs Cycle Mode."""
    ...

def ht_trendline(close: ArrayLike) -> Array1D:
    """Hilbert Transform - Instantaneous Trendline."""
    ...

# ============================================================================
# Volume Indicators
# ============================================================================

def obv(close: ArrayLike, volume: ArrayLike) -> Array1D:
    """On Balance Volume."""
    ...

def ad(high: ArrayLike, low: ArrayLike, close: ArrayLike, volume: ArrayLike) -> Array1D:
    """Chaikin A/D Line."""
    ...

def adosc(high: ArrayLike, low: ArrayLike, close: ArrayLike, volume: ArrayLike, *, fastperiod: int = 3, slowperiod: int = 10) -> Array1D:
    """Chaikin A/D Oscillator."""
    ...

# ============================================================================
# Volatility Indicators
# ============================================================================

def atr(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Average True Range."""
    ...

def natr(high: ArrayLike, low: ArrayLike, close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Normalized Average True Range."""
    ...

def trange(high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """True Range."""
    ...

# ============================================================================
# Pattern Recognition
# ============================================================================

def cdl2crows(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Two Crows."""
    ...

def cdl3blackcrows(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three Black Crows."""
    ...

def cdl3inside(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three Inside Up/Down."""
    ...

def cdl3linestrike(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three-Line Strike."""
    ...

def cdl3outside(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three Outside Up/Down."""
    ...

def cdl3starsinsouth(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three Stars In The South."""
    ...

def cdl3whitesoldiers(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Three Advancing White Soldiers."""
    ...

def cdlabandonedbaby(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.3) -> Array1D:
    """Abandoned Baby."""
    ...

def cdladvanceblock(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Advance Block."""
    ...

def cdlbelthold(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Belt-hold."""
    ...

def cdlbreakaway(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Breakaway."""
    ...

def cdlclosingmarubozu(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Closing Marubozu."""
    ...

def cdlconcealbabyswall(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Concealing Baby Swallow."""
    ...

def cdlcounterattack(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Counterattack."""
    ...

def cdldarkcloudcover(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.5) -> Array1D:
    """Dark Cloud Cover."""
    ...

def cdldoji(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Doji."""
    ...

def cdldojistar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Doji Star."""
    ...

def cdldragonflydoji(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Dragonfly Doji."""
    ...

def cdlengulfing(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Engulfing Pattern."""
    ...

def cdleveningdojistar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.3) -> Array1D:
    """Evening Doji Star."""
    ...

def cdleveningstar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.3) -> Array1D:
    """Evening Star."""
    ...

def cdlgapsidesidewhite(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Up/Down-gap side-by-side white lines."""
    ...

def cdlgravestonedoji(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Gravestone Doji."""
    ...

def cdlhammer(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Hammer."""
    ...

def cdlhangingman(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Hanging Man."""
    ...

def cdlharami(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Harami Pattern."""
    ...

def cdlharamicross(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Harami Cross Pattern."""
    ...

def cdlhighwave(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """High-Wave Candle."""
    ...

def cdlhikkake(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Hikkake Pattern."""
    ...

def cdlhikkakemod(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Hikkake Modified Pattern."""
    ...

def cdlhomingsoldier(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Homing Pigeon."""
    ...

def cdlidentical3crows(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Identical Three Crows."""
    ...

def cdlinneck(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """In-Neck Pattern."""
    ...

def cdlinvertedhammer(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Inverted Hammer."""
    ...

def cdlkicking(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Kicking."""
    ...

def cdlkickingbylength(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Kicking - bull/bear determined by the longer marubozu."""
    ...

def cdlladderbottom(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Ladder Bottom."""
    ...

def cdllongleggeddoji(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Long Legged Doji."""
    ...

def cdllongline(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Long Line Candle."""
    ...

def cdlmarubozu(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Marubozu."""
    ...

def cdlmatchinglow(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Matching Low."""
    ...

def cdlmathold(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.5) -> Array1D:
    """Mat Hold."""
    ...

def cdlmorningdojistar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.3) -> Array1D:
    """Morning Doji Star."""
    ...

def cdlmorningstar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike, *, penetration: float = 0.3) -> Array1D:
    """Morning Star."""
    ...

def cdlonneck(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """On-Neck Pattern."""
    ...

def cdlpiercing(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Piercing Pattern."""
    ...

def cdlrickshawman(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Rickshaw Man."""
    ...

def cdlrisefall3methods(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Rising/Falling Three Methods."""
    ...

def cdlseparatinglines(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Separating Lines."""
    ...

def cdlshootingstar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Shooting Star."""
    ...

def cdlshortline(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Short Line Candle."""
    ...

def cdlspinningtop(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Spinning Top."""
    ...

def cdlstalledpattern(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Stalled Pattern."""
    ...

def cdlsticksandwich(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Stick Sandwich."""
    ...

def cdltakuri(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Takuri (Dragonfly Doji with very long lower shadow)."""
    ...

def cdltasukigap(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Tasuki Gap."""
    ...

def cdlthrusting(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Thrusting Pattern."""
    ...

def cdltristar(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Tristar Pattern."""
    ...

def cdlunique3river(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Unique 3 River."""
    ...

def cdlupsidegap2crows(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Upside Gap Two Crows."""
    ...

def cdlxsidegap3methods(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Upside/Downside Gap Three Methods."""
    ...

# ============================================================================
# Statistic Functions
# ============================================================================

def beta(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 5) -> Array1D:
    """Beta."""
    ...

def correl(high: ArrayLike, low: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Pearson's Correlation Coefficient (r)."""
    ...

def linearreg(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Linear Regression."""
    ...

def linearreg_angle(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Linear Regression Angle."""
    ...

def linearreg_intercept(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Linear Regression Intercept."""
    ...

def linearreg_slope(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Linear Regression Slope."""
    ...

def stddev(close: ArrayLike, *, timeperiod: int = 5, nbdev: float = 1.0) -> Array1D:
    """Standard Deviation."""
    ...

def tsf(close: ArrayLike, *, timeperiod: int = 14) -> Array1D:
    """Time Series Forecast."""
    ...

def var(close: ArrayLike, *, timeperiod: int = 5, nbdev: float = 1.0) -> Array1D:
    """Variance."""
    ...

# ============================================================================
# Price Transform
# ============================================================================

def avgprice(open: ArrayLike, high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Average Price."""
    ...

def medprice(high: ArrayLike, low: ArrayLike) -> Array1D:
    """Median Price."""
    ...

def typprice(high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Typical Price."""
    ...

def wclprice(high: ArrayLike, low: ArrayLike, close: ArrayLike) -> Array1D:
    """Weighted Close Price."""
    ...

# ============================================================================
# Math Transform
# ============================================================================

def add(high: ArrayLike, low: ArrayLike) -> Array1D:
    """Vector Arithmetic Add."""
    ...

def div(high: ArrayLike, low: ArrayLike) -> Array1D:
    """Vector Arithmetic Div."""
    ...

def max(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Highest value over a specified period."""
    ...

def maxindex(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Index of highest value over a specified period."""
    ...

def min(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Lowest value over a specified period."""
    ...

def minindex(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Index of lowest value over a specified period."""
    ...

def minmax(close: ArrayLike, *, timeperiod: int = 30) -> Tuple[Array1D, Array1D]:
    """Lowest and highest values over a specified period. Returns (min, max)."""
    ...

def minmaxindex(close: ArrayLike, *, timeperiod: int = 30) -> Tuple[Array1D, Array1D]:
    """Indexes of lowest and highest values over a specified period. Returns (minidx, maxidx)."""
    ...

def mult(high: ArrayLike, low: ArrayLike) -> Array1D:
    """Vector Arithmetic Mult."""
    ...

def sub(high: ArrayLike, low: ArrayLike) -> Array1D:
    """Vector Arithmetic Subtraction."""
    ...

def sum(close: ArrayLike, *, timeperiod: int = 30) -> Array1D:
    """Summation."""
    ...

# ============================================================================
# Math Operators
# ============================================================================

def acos(close: ArrayLike) -> Array1D:
    """Vector Trigonometric ACos."""
    ...

def asin(close: ArrayLike) -> Array1D:
    """Vector Trigonometric ASin."""
    ...

def atan(close: ArrayLike) -> Array1D:
    """Vector Trigonometric ATan."""
    ...

def ceil(close: ArrayLike) -> Array1D:
    """Vector Ceil."""
    ...

def cos(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Cos."""
    ...

def cosh(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Cosh."""
    ...

def exp(close: ArrayLike) -> Array1D:
    """Vector Arithmetic Exp."""
    ...

def floor(close: ArrayLike) -> Array1D:
    """Vector Floor."""
    ...

def ln(close: ArrayLike) -> Array1D:
    """Vector Log Natural."""
    ...

def log10(close: ArrayLike) -> Array1D:
    """Vector Log10."""
    ...

def sin(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Sin."""
    ...

def sinh(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Sinh."""
    ...

def sqrt(close: ArrayLike) -> Array1D:
    """Vector Square Root."""
    ...

def tan(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Tan."""
    ...

def tanh(close: ArrayLike) -> Array1D:
    """Vector Trigonometric Tanh."""
    ...

# ============================================================================
# Formula Engine
# ============================================================================

class CompiledFormula:
    """Reusable formula compilation plan for repeated evaluations."""

    def __init__(self, source: str) -> None:
        ...

    @property
    def source(self) -> str:
        ...

    def eval(
        self,
        open: ArrayLike,
        high: ArrayLike,
        low: ArrayLike,
        close: ArrayLike,
        volume: ArrayLike,
        amount: Optional[ArrayLike] = ...,
    ) -> Dict[str, Array1D]:
        """Evaluate the compiled formula and return NumPy arrays."""
        ...

def formula_eval(
    source: str,
    open: ArrayLike,
    high: ArrayLike,
    low: ArrayLike,
    close: ArrayLike,
    volume: ArrayLike,
    amount: Optional[ArrayLike] = ...,
) -> Dict[str, Array1D]:
    """Compile and execute a formula once."""
    ...

def formula_eval_dialect(
    source: str,
    open: ArrayLike,
    high: ArrayLike,
    low: ArrayLike,
    close: ArrayLike,
    volume: ArrayLike,
    dialect: str = "alpha_ta",
    amount: Optional[ArrayLike] = ...,
) -> Dict[str, Array1D]:
    """Compile and execute a formula using a named dialect."""
    ...

# ============================================================================
# Visualization
# ============================================================================

class KlineChart:
    """K-line chart visualization."""

    def __init__(
        self,
        data: "KlineData",
        language: str = "zh",
        title: str = "",
        width: int = 1200,
        height: int = 600,
    ) -> None:
        ...

    def set_viewport(
        self,
        start: int = 0,
        end: int = 0,
        pixel_width: int = 1200,
        pixel_height: int = 600,
        overscan_bars: int = 0,
        follow_latest: bool = False,
    ) -> None:
        ...

    def append_kline(
        self, date: str, open: float, high: float, low: float, close: float, volume: float
    ) -> None:
        ...

    def update_last_kline(
        self,
        close: float,
        high: Optional[float] = ...,
        low: Optional[float] = ...,
        volume: Optional[float] = ...,
    ) -> None:
        ...

    def upsert_kline(
        self, date: str, open: float, high: float, low: float, close: float, volume: float
    ) -> str:
        ...

    def upsert_kline_timestamped(
        self,
        timestamp: int,
        date: str,
        open: float,
        high: float,
        low: float,
        close: float,
        volume: float,
    ) -> str:
        ...

    def upsert_klines(
        self,
        updates: List[Tuple[str, float, float, float, float, float]],
    ) -> List[str]:
        ...

    def set_lod_policy(self, level: str = "auto") -> None:
        ...

    def set_replay_window(
        self,
        window: int = 200,
        cursor: Optional[int] = ...,
        pixel_width: int = 1200,
        pixel_height: int = 600,
        overscan_bars: int = 0,
    ) -> Tuple[int, int]:
        ...

    def replay_next(self) -> Optional[Tuple[int, int]]:
        ...

    def set_layer_visible(self, layer: str, visible: bool) -> None:
        ...

    def add_ma(self, periods: List[int]) -> None:
        ...

    def add_ema(self, periods: List[int]) -> None:
        ...

    def add_boll(self, period: int = 20, nb_dev: float = 2.0) -> None:
        ...

    def add_macd(self, fast: int = 12, slow: int = 26, signal: int = 9) -> None:
        ...

    def add_rsi(self, period: int = 14) -> None:
        ...

    def add_kdj(self, fast_k: int = 9, slow_k: int = 3, slow_d: int = 3) -> None:
        ...

    def add_sar(self, acceleration: float = 0.02, maximum: float = 0.2) -> None:
        ...

    def add_custom_indicator(self, name: str, values: ArrayLike) -> None:
        ...

    def set_custom_indicator_series(self, name: str, values: ArrayLike) -> None:
        """Replace or register an aligned custom indicator series."""
        ...

    def add_event_marker(
        self,
        index: int,
        label: str,
        value: Optional[float] = ...,
        color: str = "#f59e0b",
        priority: int = 30,
    ) -> None:
        ...

    def set_interaction(
        self,
        enabled: bool = True,
        show_crosshair: bool = True,
        show_data_window: bool = True,
        enable_pan_zoom: bool = True,
        enable_keyboard: bool = True,
    ) -> None:
        ...

    def add_chan(
        self,
        min_stroke_bars: int = 6,
        show_labels: bool = False,
        variant: str = "standard",
        stroke_policy: str = "configurable",
        center_policy: str = "dynamic",
        signal_min_strength: float = 0.0,
        show_multi_timeframe_annotations: bool = False,
    ) -> None:
        ...

    def add_chan_multi(self, factors: Optional[List[int]] = ..., variant: str = "standard") -> None:
        ...

    def set_chan_thresholds(
        self,
        min_stroke_change_ratio: float,
        min_fractal_range_ratio: float,
        signal_min_strength: float,
        center_break_ratio: float,
    ) -> None:
        ...

    def save_as_svg(self, path: str) -> None:
        ...

    def save_as_html(self, path: str) -> None:
        ...

    def save_as_canvas_html(self, path: str) -> None:
        ...

    def save_as_webgl_html(self, path: str) -> None:
        ...

    def save_as_webgpu_html(self, path: str) -> None:
        ...

    def to_svg_string(self) -> str:
        ...

    def to_canvas_html(self) -> str:
        ...

    def to_webgl_html(self) -> str:
        ...

    def to_webgpu_html(self) -> str:
        ...

class KlineData:
    """Validated OHLCV container with optional Unix-second timestamps."""

    def __init__(
        self,
        dates: List[str],
        opens: ArrayLike,
        highs: ArrayLike,
        lows: ArrayLike,
        closes: ArrayLike,
        volumes: ArrayLike,
        timestamps: Optional[List[int]] = ...,
    ) -> None:
        ...

    def __len__(self) -> int:
        ...

    def validate(self) -> bool:
        ...

    def validate_ohlcv(self) -> bool:
        ...

    def validation_errors(self) -> List[str]:
        ...

    def set_timestamps(self, timestamps: List[int]) -> None:
        ...

    def push(
        self, date: str, open: float, high: float, low: float, close: float, volume: float
    ) -> None:
        ...

    def push_timestamped(
        self,
        timestamp: int,
        date: str,
        open: float,
        high: float,
        low: float,
        close: float,
        volume: float,
    ) -> None:
        ...

    @staticmethod
    def from_json(json_str: str) -> "KlineData":
        ...

    @staticmethod
    def from_csv(csv_str: str) -> "KlineData":
        ...

    @property
    def dates(self) -> List[str]:
        ...

    @property
    def opens(self) -> List[float]:
        ...

    @property
    def highs(self) -> List[float]:
        ...

    @property
    def lows(self) -> List[float]:
        ...

    @property
    def closes(self) -> List[float]:
        ...

    @property
    def volumes(self) -> List[float]:
        ...

    @property
    def timestamps(self) -> List[int]:
        ...

# ============================================================================
# Streaming Indicators
# ============================================================================

class StreamingIndicator:
    """Base class for streaming indicators."""
    
    def next(self, value: float) -> Optional[float]:
        ...
    
    def reset(self) -> None:
        ...

class StreamingSMA(StreamingIndicator):
    """Streaming Simple Moving Average."""
    
    def __init__(self, period: int) -> None:
        ...

class StreamingEMA(StreamingIndicator):
    """Streaming Exponential Moving Average."""
    
    def __init__(self, period: int) -> None:
        ...

class MACDResult:
    macd: float
    signal: float
    histogram: float

class StreamingMACDEXT:
    """Streaming MACD with configurable scalar MA types."""

    def __init__(
        self,
        fast_period: int = 12,
        slow_period: int = 26,
        signal_period: int = 9,
        fast_ma: str = "ema",
        slow_ma: str = "ema",
        signal_ma: str = "ema",
    ) -> None:
        ...

    def update(self, value: float) -> MACDResult:
        ...

    def update_batch(self, values: Sequence[float]) -> List[MACDResult]:
        ...

    def reset(self) -> None:
        ...

    def is_ready(self) -> bool:
        ...

    def count(self) -> int:
        ...

class StreamingRSI(StreamingIndicator):
    """Streaming Relative Strength Index."""
    
    def __init__(self, period: int) -> None:
        ...

# ============================================================================
# Exceptions
# ============================================================================

class FinkitError(Exception):
    """Base exception for finkit errors."""
    ...

class InsufficientDataError(FinkitError):
    """Raised when there is insufficient data for the calculation."""
    ...

class InvalidParameterError(FinkitError):
    """Raised when an invalid parameter is provided."""
    ...

class IndicatorNotFoundError(FinkitError):
    """Raised when an indicator is not found."""
    ...

# ============================================================================
# Accessor Registration
# ============================================================================

def register_accessor() -> None:
    """Register the pandas DataFrame accessor (df.ta)."""
    ...

# ============================================================================
# Module Exports
# ============================================================================

__all__ = [
    # Overlap Studies
    "sma", "ema", "wma", "dema", "tema", "kama", "mama", "t3",
    "bollinger_bands", "sar", "midpoint", "midprice",
    # Momentum Indicators
    "rsi", "macd", "stoch", "adx", "aroon", "cci", "mom", "roc", "willr",
    "apo", "bop", "cmo", "dx", "mfi", "minus_di", "minus_dm", "plus_di",
    "plus_dm", "trix",
    # Cycle Indicators
    "ht_dcperiod", "ht_dcphase", "ht_phasor", "ht_sine", "ht_trendmode",
    "ht_trendline",
    # Volume Indicators
    "obv", "ad", "adosc",
    # Volatility Indicators
    "atr", "natr", "trange",
    # Pattern Recognition
    "cdl2crows", "cdl3blackcrows", "cdl3inside", "cdl3linestrike",
    "cdl3outside", "cdl3starsinsouth", "cdl3whitesoldiers",
    "cdlabandonedbaby", "cdladvanceblock", "cdlbelthold", "cdlbreakaway",
    "cdlclosingmarubozu", "cdlconcealbabyswall", "cdlcounterattack",
    "cdldarkcloudcover", "cdldoji", "cdldojistar", "cdldragonflydoji",
    "cdlengulfing", "cdleveningdojistar", "cdleveningstar",
    "cdlgapsidesidewhite", "cdlgravestonedoji", "cdlhammer", "cdlhangingman",
    "cdlharami", "cdlharamicross", "cdlhighwave", "cdlhikkake",
    "cdlhikkakemod", "cdlhomingsoldier", "cdlidentical3crows", "cdlinneck",
    "cdlinvertedhammer", "cdlkicking", "cdlkickingbylength", "cdlladderbottom",
    "cdllongleggeddoji", "cdllongline", "cdlmarubozu", "cdlmatchinglow",
    "cdlmathold", "cdlmorningdojistar", "cdlmorningstar", "cdlonneck",
    "cdlpiercing", "cdlrickshawman", "cdlrisefall3methods", "cdlseparatinglines",
    "cdlshootingstar", "cdlshortline", "cdlspinningtop", "cdlstalledpattern",
    "cdlsticksandwich", "cdltakuri", "cdltasukigap", "cdlthrusting",
    "cdltristar", "cdlunique3river", "cdlupsidegap2crows", "cdlxsidegap3methods",
    # Statistic Functions
    "beta", "correl", "linearreg", "linearreg_angle", "linearreg_intercept",
    "linearreg_slope", "stddev", "tsf", "var",
    # Price Transform
    "avgprice", "medprice", "typprice", "wclprice",
    # Math Transform
    "add", "div", "max", "maxindex", "min", "minindex", "minmax", "minmaxindex",
    "mult", "sub", "sum",
    # Math Operators
    "acos", "asin", "atan", "ceil", "cos", "cosh", "exp", "floor", "ln",
    "log10", "sin", "sinh", "sqrt", "tan", "tanh",
    # Formula Engine
    "CompiledFormula", "formula_eval", "formula_eval_dialect",
    # Visualization
    "KlineChart", "KlineData",
    # Streaming Indicators
    "StreamingIndicator", "StreamingSMA", "StreamingEMA", "StreamingMACDEXT", "StreamingRSI",
    # Exceptions
    "FinkitError", "InsufficientDataError", "InvalidParameterError",
    "IndicatorNotFoundError",
    # Accessor
    "register_accessor",
]
