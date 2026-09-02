using System.Runtime.InteropServices;
using System.Text.Json;

namespace Finkit;

/// <summary>
/// Provides technical analysis indicators powered by Rust TA-Lib.
/// All indicators use P/Invoke to call the native Rust library for maximum performance.
/// </summary>
public static class Indicators
{
    private const string LibraryName = "finkit_dotnet";

    static Indicators()
    {
        NativeLibraryResolver.EnsureLibraryLoaded();
    }

    // ========================================================================
    // P/Invoke Signatures
    // ========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_sma(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ema(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_wma(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_dema(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_tema(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_kama(IntPtr input, int length, int period, int fastPeriod, int slowPeriod, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_t3(IntPtr input, int length, int period, double vfactor, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_rsi(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_macd(IntPtr input, int length, int fastPeriod, int slowPeriod, int signalPeriod, IntPtr outMacd, IntPtr outSignal, IntPtr outHist);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_stoch(IntPtr high, IntPtr low, IntPtr close, int length, int kPeriod, int kSlow, int dPeriod, IntPtr outK, IntPtr outD);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_adx(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_aroon(IntPtr high, IntPtr low, int length, int period, IntPtr outUp, IntPtr outDown);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cci(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_mom(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_roc(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_willr(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_obv(IntPtr close, IntPtr volume, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ad(IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ad_osc(IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length, int fastPeriod, int slowPeriod, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_atr(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_natr(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_trange(IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_bbands(IntPtr input, int length, int period, double nbDevUp, double nbDevDn, IntPtr outUpper, IntPtr outMiddle, IntPtr outLower);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_dcperiod(IntPtr input, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_dcphase(IntPtr input, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_phasor(IntPtr input, int length, IntPtr outInphase, IntPtr outQuadrature);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_sine(IntPtr input, int length, IntPtr outSine, IntPtr outLead);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_trendmode(IntPtr input, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_ht_trendline(IntPtr input, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_zscore(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_beta(IntPtr asset, IntPtr benchmark, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_correlation(IntPtr inputA, IntPtr inputB, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_std_dev(IntPtr input, int length, int period, double nbDev, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_linear_reg(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_tsf(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_mama(IntPtr input, int length, double fastLimit, double slowLimit, IntPtr outMama, IntPtr outFama);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr ta_formula_eval(string source, IntPtr open, IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr ta_formula_eval_jit(string source, IntPtr open, IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr ta_formula_eval_simd(string source, IntPtr open, IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr ta_formula_eval_zc_exec(string source, IntPtr open, IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_formula_validate(string source);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern void ta_free_string(IntPtr s);

    // ========================================================================
    // Helper Methods
    // ========================================================================

    private static unsafe void CopyToArray(double[] source, IntPtr dest, int length)
    {
        fixed (double* p = source)
        {
            Buffer.MemoryCopy(p, (void*)dest, length * sizeof(double), source.Length * sizeof(double));
        }
    }

    private static double[] AllocateAndInitialize(int length)
    {
        return new double[length];
    }

    // ========================================================================
    // Overlap Studies
    // ========================================================================

    /// <summary>
    /// Simple Moving Average (SMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <returns>Array of SMA values. Initial values may be NaN.</returns>
    public static double[] Sma(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_sma((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"SMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Exponential Moving Average (EMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <returns>Array of EMA values. Initial values may be NaN.</returns>
    public static double[] Ema(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_ema((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"EMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Weighted Moving Average (WMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <returns>Array of WMA values. Initial values may be NaN.</returns>
    public static double[] Wma(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_wma((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"WMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Double Exponential Moving Average (DEMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <returns>Array of DEMA values. Initial values may be NaN.</returns>
    public static double[] Dema(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_dema((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"DEMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Triple Exponential Moving Average (TEMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <returns>Array of TEMA values. Initial values may be NaN.</returns>
    public static double[] Tema(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_tema((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"TEMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Kaufman Adaptive Moving Average (KAMA).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <param name="fastPeriod">Fast period for efficiency ratio (default 2).</param>
    /// <param name="slowPeriod">Slow period for efficiency ratio (default 30).</param>
    /// <returns>Array of KAMA values. Initial values may be NaN.</returns>
    public static double[] Kama(double[] input, int period, int fastPeriod = 2, int slowPeriod = 30)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_kama((IntPtr)pIn, input.Length, period, fastPeriod, slowPeriod, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"KAMA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// T3 Moving Average (T3).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <param name="vfactor">Volume factor (0 to 1, default 0.7). Lower values are smoother, higher values more responsive.</param>
    /// <returns>Array of T3 values. Initial values may be NaN.</returns>
    public static double[] T3(double[] input, int period, double vfactor = 0.7)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_t3((IntPtr)pIn, input.Length, period, vfactor, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"T3 failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Bollinger Bands (BBANDS).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period.</param>
    /// <param name="nbDevUp">Number of standard deviations for upper band (default 2.0).</param>
    /// <param name="nbDevDn">Number of standard deviations for lower band (default 2.0).</param>
    /// <returns>BbandsResult containing upper, middle, and lower bands.</returns>
    public static BbandsResult Bbands(double[] input, int period, double nbDevUp = 2.0, double nbDevDn = 2.0)
    {
        var upper = AllocateAndInitialize(input.Length);
        var middle = AllocateAndInitialize(input.Length);
        var lower = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pUpper = upper)
            fixed (double* pMiddle = middle)
            fixed (double* pLower = lower)
            {
                int result = ta_bbands((IntPtr)pIn, input.Length, period, nbDevUp, nbDevDn, (IntPtr)pUpper, (IntPtr)pMiddle, (IntPtr)pLower);
                if (result != 0) throw new InvalidOperationException($"BBANDS failed with error code {result}");
            }
        }
        return new BbandsResult(upper, middle, lower);
    }

    // ========================================================================
    // Momentum Indicators
    // ========================================================================

    /// <summary>
    /// Relative Strength Index (RSI).
    /// </summary>
    /// <param name="input">Input data series (typically close prices).</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of RSI values (0-100 range). Initial values may be NaN.</returns>
    public static double[] Rsi(double[] input, int period = 14)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_rsi((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"RSI failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Moving Average Convergence Divergence (MACD).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="fastPeriod">Fast EMA period (default 12).</param>
    /// <param name="slowPeriod">Slow EMA period (default 26).</param>
    /// <param name="signalPeriod">Signal line EMA period (default 9).</param>
    /// <returns>MacdResult containing MACD, signal, and histogram arrays.</returns>
    public static MacdResult Macd(double[] input, int fastPeriod = 12, int slowPeriod = 26, int signalPeriod = 9)
    {
        var macd = AllocateAndInitialize(input.Length);
        var signal = AllocateAndInitialize(input.Length);
        var hist = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pMacd = macd)
            fixed (double* pSignal = signal)
            fixed (double* pHist = hist)
            {
                int result = ta_macd((IntPtr)pIn, input.Length, fastPeriod, slowPeriod, signalPeriod, (IntPtr)pMacd, (IntPtr)pSignal, (IntPtr)pHist);
                if (result != 0) throw new InvalidOperationException($"MACD failed with error code {result}");
            }
        }
        return new MacdResult(macd, signal, hist);
    }

    /// <summary>
    /// Stochastic Oscillator (STOCH).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="kPeriod">%K lookback period (default 14).</param>
    /// <param name="kSlow">%K slowing period (default 3).</param>
    /// <param name="dPeriod">%D period (default 3).</param>
    /// <returns>StochResult containing %K and %D arrays.</returns>
    public static StochResult Stoch(double[] high, double[] low, double[] close, int kPeriod = 14, int kSlow = 3, int dPeriod = 3)
    {
        var k = AllocateAndInitialize(high.Length);
        var d = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pK = k)
            fixed (double* pD = d)
            {
                int result = ta_stoch((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, kPeriod, kSlow, dPeriod, (IntPtr)pK, (IntPtr)pD);
                if (result != 0) throw new InvalidOperationException($"STOCH failed with error code {result}");
            }
        }
        return new StochResult(k, d);
    }

    /// <summary>
    /// Average Directional Index (ADX).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of ADX values. Initial values may be NaN.</returns>
    public static double[] Adx(double[] high, double[] low, double[] close, int period = 14)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_adx((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"ADX failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Aroon Indicator (AROON).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>AroonResult containing Aroon Up and Aroon Down arrays.</returns>
    public static AroonResult Aroon(double[] high, double[] low, int period = 14)
    {
        var up = AllocateAndInitialize(high.Length);
        var down = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pUp = up)
            fixed (double* pDown = down)
            {
                int result = ta_aroon((IntPtr)pHigh, (IntPtr)pLow, high.Length, period, (IntPtr)pUp, (IntPtr)pDown);
                if (result != 0) throw new InvalidOperationException($"AROON failed with error code {result}");
            }
        }
        return new AroonResult(up, down);
    }

    /// <summary>
    /// Commodity Channel Index (CCI).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of CCI values. Initial values may be NaN.</returns>
    public static double[] Cci(double[] high, double[] low, double[] close, int period = 14)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_cci((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"CCI failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Momentum (MOM).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period (default 10).</param>
    /// <returns>Array of momentum values. Initial values may be NaN.</returns>
    public static double[] Mom(double[] input, int period = 10)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_mom((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"MOM failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Rate of Change (ROC).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Lookback period (default 10).</param>
    /// <returns>Array of ROC values (in percentage). Initial values may be NaN.</returns>
    public static double[] Roc(double[] input, int period = 10)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_roc((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"ROC failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Williams %R (WILLR).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of Williams %R values (-100 to 0 range). Initial values may be NaN.</returns>
    public static double[] Willr(double[] high, double[] low, double[] close, int period = 14)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_willr((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"WILLR failed with error code {result}");
            }
        }
        return output;
    }

    // ========================================================================
    // Volume Indicators
    // ========================================================================

    /// <summary>
    /// On Balance Volume (OBV).
    /// </summary>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Array of OBV values.</returns>
    public static double[] Obv(double[] close, double[] volume)
    {
        var output = AllocateAndInitialize(close.Length);
        unsafe
        {
            fixed (double* pClose = close)
            fixed (double* pVol = volume)
            fixed (double* pOut = output)
            {
                int result = ta_obv((IntPtr)pClose, (IntPtr)pVol, close.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"OBV failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Accumulation/Distribution Line (AD).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Array of AD values.</returns>
    public static double[] Ad(double[] high, double[] low, double[] close, double[] volume)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVol = volume)
            fixed (double* pOut = output)
            {
                int result = ta_ad((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVol, high.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"AD failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Chaikin A/D Oscillator (ADOSC).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <param name="fastPeriod">Fast EMA period (default 3).</param>
    /// <param name="slowPeriod">Slow EMA period (default 10).</param>
    /// <returns>Array of ADOSC values. Initial values may be NaN.</returns>
    public static double[] AdOsc(double[] high, double[] low, double[] close, double[] volume, int fastPeriod = 3, int slowPeriod = 10)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVol = volume)
            fixed (double* pOut = output)
            {
                int result = ta_ad_osc((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVol, high.Length, fastPeriod, slowPeriod, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"ADOSC failed with error code {result}");
            }
        }
        return output;
    }

    // ========================================================================
    // Volatility Indicators
    // ========================================================================

    /// <summary>
    /// Average True Range (ATR).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of ATR values. Initial values may be NaN.</returns>
    public static double[] Atr(double[] high, double[] low, double[] close, int period = 14)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_atr((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"ATR failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Normalized Average True Range (NATR).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="period">Lookback period (default 14).</param>
    /// <returns>Array of NATR values (in percentage). Initial values may be NaN.</returns>
    public static double[] Natr(double[] high, double[] low, double[] close, int period = 14)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_natr((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"NATR failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// True Range (TRANGE).
    /// </summary>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <returns>Array of True Range values.</returns>
    public static double[] Trange(double[] high, double[] low, double[] close)
    {
        var output = AllocateAndInitialize(high.Length);
        unsafe
        {
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pOut = output)
            {
                int result = ta_trange((IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, high.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"TRANGE failed with error code {result}");
            }
        }
        return output;
    }

    // ========================================================================
    // Hilbert Transform Indicators
    // ========================================================================

    /// <summary>
    /// Hilbert Transform - Dominant Cycle Period (HT_DCPERIOD).
    /// </summary>
    /// <param name="input">Input data series (typically typical price).</param>
    /// <returns>Array of dominant cycle period values. Initial values may be NaN (32 bars minimum).</returns>
    public static double[] HtDcPeriod(double[] input)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_ht_dcperiod((IntPtr)pIn, input.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"HT_DCPERIOD failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Hilbert Transform - Dominant Cycle Phase (HT_DCPHASE).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <returns>Array of dominant cycle phase values in degrees. Initial values may be NaN (32 bars minimum).</returns>
    public static double[] HtDcPhase(double[] input)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_ht_dcphase((IntPtr)pIn, input.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"HT_DCPHASE failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Hilbert Transform - Phasor Components (HT_PHASOR).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <returns>HtPhasorResult containing in-phase and quadrature arrays. Initial values may be NaN (12 bars minimum).</returns>
    public static HtPhasorResult HtPhasor(double[] input)
    {
        var inPhase = AllocateAndInitialize(input.Length);
        var quadrature = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pInPhase = inPhase)
            fixed (double* pQuad = quadrature)
            {
                int result = ta_ht_phasor((IntPtr)pIn, input.Length, (IntPtr)pInPhase, (IntPtr)pQuad);
                if (result != 0) throw new InvalidOperationException($"HT_PHASOR failed with error code {result}");
            }
        }
        return new HtPhasorResult(inPhase, quadrature);
    }

    /// <summary>
    /// Hilbert Transform - Sine Wave (HT_SINE).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <returns>HtSineResult containing sine and lead sine arrays. Initial values may be NaN (32 bars minimum).</returns>
    public static HtSineResult HtSine(double[] input)
    {
        var sine = AllocateAndInitialize(input.Length);
        var leadSine = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pSine = sine)
            fixed (double* pLead = leadSine)
            {
                int result = ta_ht_sine((IntPtr)pIn, input.Length, (IntPtr)pSine, (IntPtr)pLead);
                if (result != 0) throw new InvalidOperationException($"HT_SINE failed with error code {result}");
            }
        }
        return new HtSineResult(sine, leadSine);
    }

    /// <summary>
    /// Hilbert Transform - Trend vs Cycle Mode (HT_TRENDMODE).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <returns>Array of mode values (1.0 for trend, 0.0 for cycle). Initial values may be NaN (32 bars minimum).</returns>
    public static double[] HtTrendMode(double[] input)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_ht_trendmode((IntPtr)pIn, input.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"HT_TRENDMODE failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Hilbert Transform - Instantaneous Trendline (HT_TRENDLINE).
    /// </summary>
    /// <param name="input">Input data series (typically typical price).</param>
    /// <returns>Array of trendline values. Initial values may be NaN (32 bars minimum).</returns>
    public static double[] HtTrendLine(double[] input)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_ht_trendline((IntPtr)pIn, input.Length, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"HT_TRENDLINE failed with error code {result}");
            }
        }
        return output;
    }

    // ========================================================================
    // Statistics Indicators
    // ========================================================================

    /// <summary>
    /// Z-Score (Z-Score / Standardization).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Rolling window size.</param>
    /// <returns>Z-Score array. Initial values may be NaN.</returns>
    public static double[] ZScore(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_zscore((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"ZSCORE failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Beta Coefficient (BETA).
    /// </summary>
    /// <param name="asset">Asset price series (e.g., individual stock).</param>
    /// <param name="benchmark">Benchmark price series (e.g., market index).</param>
    /// <param name="period">Rolling window size.</param>
    /// <returns>Beta coefficient array. Initial values may be NaN.</returns>
    public static double[] Beta(double[] asset, double[] benchmark, int period)
    {
        var output = AllocateAndInitialize(asset.Length);
        unsafe
        {
            fixed (double* pAsset = asset)
            fixed (double* pBenchmark = benchmark)
            fixed (double* pOut = output)
            {
                int result = ta_beta((IntPtr)pAsset, (IntPtr)pBenchmark, asset.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"BETA failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Pearson Correlation Coefficient (CORRELATION).
    /// </summary>
    /// <param name="inputA">First data series.</param>
    /// <param name="inputB">Second data series.</param>
    /// <param name="period">Rolling window size.</param>
    /// <returns>Correlation array in range [-1, 1]. Initial values may be NaN.</returns>
    public static double[] Correlation(double[] inputA, double[] inputB, int period)
    {
        var output = AllocateAndInitialize(inputA.Length);
        unsafe
        {
            fixed (double* pA = inputA)
            fixed (double* pB = inputB)
            fixed (double* pOut = output)
            {
                int result = ta_correlation((IntPtr)pA, (IntPtr)pB, inputA.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"CORRELATION failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Standard Deviation (STDDEV).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Rolling window size.</param>
    /// <param name="nbDev">Number of standard deviations (default 1.0).</param>
    /// <returns>Standard deviation array. Initial values may be NaN.</returns>
    public static double[] StdDev(double[] input, int period, double nbDev = 1.0)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_std_dev((IntPtr)pIn, input.Length, period, nbDev, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"STDDEV failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Linear Regression (LINEAR_REG).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Rolling window size.</param>
    /// <returns>Linear regression predicted values. Initial values may be NaN.</returns>
    public static double[] LinearReg(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_linear_reg((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"LINEAR_REG failed with error code {result}");
            }
        }
        return output;
    }

    /// <summary>
    /// Time Series Forecast (TSF).
    /// </summary>
    /// <param name="input">Input data series.</param>
    /// <param name="period">Rolling window size.</param>
    /// <returns>Time series forecast values. Initial values may be NaN.</returns>
    public static double[] Tsf(double[] input, int period)
    {
        var output = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pOut = output)
            {
                int result = ta_tsf((IntPtr)pIn, input.Length, period, (IntPtr)pOut);
                if (result != 0) throw new InvalidOperationException($"TSF failed with error code {result}");
            }
        }
        return output;
    }

    // ========================================================================
    // MAMA (MESA Adaptive Moving Average)
    // ========================================================================

    /// <summary>
    /// MESA Adaptive Moving Average (MAMA).
    /// </summary>
    /// <param name="input">Input data series (at least 7 points).</param>
    /// <param name="fastLimit">Fast limit for alpha (default 0.5).</param>
    /// <param name="slowLimit">Slow limit for alpha (default 0.05).</param>
    /// <returns>MamaResult containing MAMA and FAMA arrays.</returns>
    public static MamaResult Mama(double[] input, double fastLimit = 0.5, double slowLimit = 0.05)
    {
        var mama = AllocateAndInitialize(input.Length);
        var fama = AllocateAndInitialize(input.Length);
        unsafe
        {
            fixed (double* pIn = input)
            fixed (double* pMama = mama)
            fixed (double* pFama = fama)
            {
                int result = ta_mama((IntPtr)pIn, input.Length, fastLimit, slowLimit, (IntPtr)pMama, (IntPtr)pFama);
                if (result != 0) throw new InvalidOperationException($"MAMA failed with error code {result}");
            }
        }
        return new MamaResult(mama, fama);
    }

    // ========================================================================
    // Formula Engine
    // ========================================================================

    /// <summary>
    /// Evaluates a formula string against OHLCV data.
    /// </summary>
    /// <param name="source">Formula source code string.</param>
    /// <param name="open">Open prices.</param>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Dictionary of variable names to their computed arrays, including "__final__".</returns>
    /// <exception cref="InvalidOperationException">Thrown if evaluation fails.</exception>
    public static Dictionary<string, double[]> FormulaEval(string source, double[] open, double[] high, double[] low, double[] close, double[] volume)
    {
        int length = open.Length;
        if (high.Length != length || low.Length != length || close.Length != length || volume.Length != length)
            throw new ArgumentException("All input arrays must have the same length");

        unsafe
        {
            fixed (double* pOpen = open)
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVolume = volume)
            {
                IntPtr resultPtr = ta_formula_eval(source, (IntPtr)pOpen, (IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVolume, length);
                try
                {
                    string resultStr = Marshal.PtrToStringAnsi(resultPtr) ?? "";

                    if (resultStr.StartsWith("error:"))
                        throw new InvalidOperationException(resultStr.Substring(6).Trim());

                    var options = new JsonSerializerOptions
                    {
                        PropertyNameCaseInsensitive = true
                    };
                    var dict = JsonSerializer.Deserialize<Dictionary<string, List<double?>>>(resultStr, options);

                    if (dict == null)
                        throw new InvalidOperationException("Failed to deserialize formula result");

                    var result = new Dictionary<string, double[]>();
                    foreach (var kvp in dict)
                    {
                        var arr = new double[kvp.Value.Count];
                        for (int i = 0; i < kvp.Value.Count; i++)
                        {
                            arr[i] = kvp.Value[i] ?? double.NaN;
                        }
                        result[kvp.Key] = arr;
                    }
                    return result;
                }
                finally
                {
                    ta_free_string(resultPtr);
                }
            }
        }
    }

    /// <summary>
    /// Validates a formula source string for syntactic correctness.
    /// </summary>
    /// <param name="source">Formula source code string.</param>
    /// <returns>true if the formula is valid, false otherwise.</returns>
    public static bool FormulaValidate(string source)
    {
        return ta_formula_validate(source) == 1;
    }

    /// <summary>
    /// Evaluates a formula string with JIT compilation.
    /// </summary>
    /// <param name="source">Formula source code string.</param>
    /// <param name="open">Open prices.</param>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Dictionary of variable names to their computed arrays, including "__final__".</returns>
    /// <exception cref="InvalidOperationException">Thrown if evaluation fails.</exception>
    public static Dictionary<string, double[]> FormulaEvalJit(string source, double[] open, double[] high, double[] low, double[] close, double[] volume)
    {
        int length = open.Length;
        if (high.Length != length || low.Length != length || close.Length != length || volume.Length != length)
            throw new ArgumentException("All input arrays must have the same length");

        unsafe
        {
            fixed (double* pOpen = open)
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVolume = volume)
            {
                IntPtr resultPtr = ta_formula_eval_jit(source, (IntPtr)pOpen, (IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVolume, length);
                try
                {
                    string resultStr = Marshal.PtrToStringAnsi(resultPtr) ?? "";

                    if (resultStr.StartsWith("error:"))
                        throw new InvalidOperationException(resultStr.Substring(6).Trim());

                    var options = new JsonSerializerOptions
                    {
                        PropertyNameCaseInsensitive = true
                    };
                    var dict = JsonSerializer.Deserialize<Dictionary<string, List<double?>>>(resultStr, options);

                    if (dict == null)
                        throw new InvalidOperationException("Failed to deserialize formula result");

                    var result = new Dictionary<string, double[]>();
                    foreach (var kvp in dict)
                    {
                        var arr = new double[kvp.Value.Count];
                        for (int i = 0; i < kvp.Value.Count; i++)
                        {
                            arr[i] = kvp.Value[i] ?? double.NaN;
                        }
                        result[kvp.Key] = arr;
                    }
                    return result;
                }
                finally
                {
                    ta_free_string(resultPtr);
                }
            }
        }
    }

    /// <summary>
    /// Evaluates a formula string with SIMD optimization.
    /// </summary>
    /// <param name="source">Formula source code string.</param>
    /// <param name="open">Open prices.</param>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Dictionary of variable names to their computed arrays, including "__final__".</returns>
    /// <exception cref="InvalidOperationException">Thrown if evaluation fails.</exception>
    public static Dictionary<string, double[]> FormulaEvalSimd(string source, double[] open, double[] high, double[] low, double[] close, double[] volume)
    {
        int length = open.Length;
        if (high.Length != length || low.Length != length || close.Length != length || volume.Length != length)
            throw new ArgumentException("All input arrays must have the same length");

        unsafe
        {
            fixed (double* pOpen = open)
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVolume = volume)
            {
                IntPtr resultPtr = ta_formula_eval_simd(source, (IntPtr)pOpen, (IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVolume, length);
                try
                {
                    string resultStr = Marshal.PtrToStringAnsi(resultPtr) ?? "";

                    if (resultStr.StartsWith("error:"))
                        throw new InvalidOperationException(resultStr.Substring(6).Trim());

                    var options = new JsonSerializerOptions
                    {
                        PropertyNameCaseInsensitive = true
                    };
                    var dict = JsonSerializer.Deserialize<Dictionary<string, List<double?>>>(resultStr, options);

                    if (dict == null)
                        throw new InvalidOperationException("Failed to deserialize formula result");

                    var result = new Dictionary<string, double[]>();
                    foreach (var kvp in dict)
                    {
                        var arr = new double[kvp.Value.Count];
                        for (int i = 0; i < kvp.Value.Count; i++)
                        {
                            arr[i] = kvp.Value[i] ?? double.NaN;
                        }
                        result[kvp.Key] = arr;
                    }
                    return result;
                }
                finally
                {
                    ta_free_string(resultPtr);
                }
            }
        }
    }

    /// <summary>
    /// Evaluates a formula string with zero-copy optimization.
    /// </summary>
    /// <param name="source">Formula source code string.</param>
    /// <param name="open">Open prices.</param>
    /// <param name="high">High prices.</param>
    /// <param name="low">Low prices.</param>
    /// <param name="close">Close prices.</param>
    /// <param name="volume">Volume data.</param>
    /// <returns>Dictionary of variable names to their computed arrays, including "__final__".</returns>
    /// <exception cref="InvalidOperationException">Thrown if evaluation fails.</exception>
    public static Dictionary<string, double[]> FormulaEvalZeroCopy(string source, double[] open, double[] high, double[] low, double[] close, double[] volume)
    {
        int length = open.Length;
        if (high.Length != length || low.Length != length || close.Length != length || volume.Length != length)
            throw new ArgumentException("All input arrays must have the same length");

        unsafe
        {
            fixed (double* pOpen = open)
            fixed (double* pHigh = high)
            fixed (double* pLow = low)
            fixed (double* pClose = close)
            fixed (double* pVolume = volume)
            {
                IntPtr resultPtr = ta_formula_eval_zc_exec(source, (IntPtr)pOpen, (IntPtr)pHigh, (IntPtr)pLow, (IntPtr)pClose, (IntPtr)pVolume, length);
                try
                {
                    string resultStr = Marshal.PtrToStringAnsi(resultPtr) ?? "";

                    if (resultStr.StartsWith("error:"))
                        throw new InvalidOperationException(resultStr.Substring(6).Trim());

                    var options = new JsonSerializerOptions
                    {
                        PropertyNameCaseInsensitive = true
                    };
                    var dict = JsonSerializer.Deserialize<Dictionary<string, List<double?>>>(resultStr, options);

                    if (dict == null)
                        throw new InvalidOperationException("Failed to deserialize formula result");

                    var result = new Dictionary<string, double[]>();
                    foreach (var kvp in dict)
                    {
                        var arr = new double[kvp.Value.Count];
                        for (int i = 0; i < kvp.Value.Count; i++)
                        {
                            arr[i] = kvp.Value[i] ?? double.NaN;
                        }
                        result[kvp.Key] = arr;
                    }
                    return result;
                }
                finally
                {
                    ta_free_string(resultPtr);
                }
            }
        }
    }
}

/// <summary>
/// Handles native library loading for cross-platform support.
/// </summary>
internal static class NativeLibraryResolver
{
    private static bool _loaded;

    public static void EnsureLibraryLoaded()
    {
        if (_loaded) return;

        var libraryName = OperatingSystem.IsWindows() ? "finkit_dotnet.dll"
                       : OperatingSystem.IsMacOS() ? "libfinkit_dotnet.dylib"
                       : "libfinkit_dotnet.so";

        // Try to load from the native directory next to the assembly
        var assemblyDir = AppContext.BaseDirectory;
        var nativeDir = Path.Combine(assemblyDir, "native");

        var runtime = GetRuntimeIdentifier();
        var arch = GetRuntimeArchitecture();
        var platformNativeDir = Path.Combine(nativeDir, runtime, arch);

        if (Directory.Exists(platformNativeDir))
        {
            var libraryPath = Path.Combine(platformNativeDir, libraryName);
            if (File.Exists(libraryPath))
            {
                NativeLibrary.Load(libraryPath);
                _loaded = true;
                return;
            }
        }

        // Try loading from standard search paths
        if (NativeLibrary.TryLoad(libraryName, typeof(Indicators).Assembly, DllImportSearchPath.ApplicationDirectory | DllImportSearchPath.AssemblyDirectory, out _))
        {
            _loaded = true;
        }

        _loaded = true;
    }

    private static string GetRuntimeIdentifier()
    {
        if (OperatingSystem.IsWindows()) return "win";
        if (OperatingSystem.IsLinux()) return "linux";
        if (OperatingSystem.IsMacOS()) return "osx";
        return "unknown";
    }

    private static string GetRuntimeArchitecture()
    {
        return RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.X86 => "x86",
            Architecture.Arm64 => "arm64",
            Architecture.Arm => "arm",
            _ => "unknown"
        };
    }
}
