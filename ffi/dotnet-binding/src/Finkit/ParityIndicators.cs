using System.Runtime.InteropServices;

namespace Finkit;

/// <summary>
/// Registry-parity indicators for the .NET binding.
///
/// <see cref="Indicators"/> declares the indicators the .NET binding shipped
/// with. This class adds the ones it was missing relative to the FFI registry,
/// so the .NET binding exposes the same 78-indicator surface as the C binding.
/// The ABI (0 = success, negative = failure) and the buffer-zeroing behaviour
/// match <see cref="Indicators"/> exactly.
///
/// Audited by: <c>python3 scripts/audit_binding_parity.py</c>
/// </summary>
public static class ParityIndicators
{
    private const string LibraryName = "finkit_dotnet";

    static ParityIndicators()
    {
        NativeLibraryResolver.EnsureLibraryLoaded();
    }

    // ========================================================================
    // P/Invoke signatures
    // ========================================================================

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_apo(IntPtr input, int length, int fastPeriod, int slowPeriod, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cmo(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_trix(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_percent_rank(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_chande_forecast(IntPtr close, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_inertia(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, int rviPeriod, int linregPeriod, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_sar(IntPtr high, IntPtr low, int length, double acceleration, double maximum, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_avgprice(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_medprice(IntPtr high, IntPtr low, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_typprice(IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_wclprice(IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_midpoint(IntPtr input, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_midprice(IntPtr high, IntPtr low, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_bop(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_mfi(IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_vzo(IntPtr close, IntPtr volume, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_volume_momentum(IntPtr volume, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_volume_roc(IntPtr volume, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_twiggs_mf(IntPtr high, IntPtr low, IntPtr close, IntPtr volume, int length, int period, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_vortex(IntPtr high, IntPtr low, IntPtr close, int length, int period, IntPtr outPlus, IntPtr outMinus);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_doji(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, double pct, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_dragonfly_doji(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, double pct, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_gravestone_doji(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, double pct, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_long_legged_doji(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, double pct, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_marubozu(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, double pct, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_hammer(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_inverted_hammer(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_hanging_man(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_shooting_star(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_engulfing(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_harami(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_morning_star(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_evening_star(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_three_white_soldiers(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    [DllImport(LibraryName, CallingConvention = CallingConvention.Cdecl)]
    private static extern int ta_cdl_three_black_crows(IntPtr open, IntPtr high, IntPtr low, IntPtr close, int length, IntPtr out_);

    // ========================================================================
    // Shared call plumbing
    // ========================================================================

    private static double[] Allocate(int length) => new double[length];

    private static int[] AllocateInt(int length) => new int[length];

    private static unsafe void CopyToArray(IntPtr source, double[] destination, int length)
    {
        fixed (double* p = destination)
        {
            Buffer.MemoryCopy((void*)source, p, length * sizeof(double), length * sizeof(double));
        }
    }

    /// <summary>
    /// Runs a one-input / one-output kernel and returns the result buffer.
    /// </summary>
    private static unsafe double[] Call1(double[] input, Func<IntPtr, int, IntPtr, int> invoke)
    {
        var output = Allocate(input.Length);
        fixed (double* pIn = input)
        fixed (double* pOut = output)
        {
            int code = invoke((IntPtr)pIn, input.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"indicator failed with error code {code}");
        }
        return output;
    }

    /// <summary>
    /// Runs an OHLC-input / one-output kernel and returns the result buffer.
    /// </summary>
    private static unsafe double[] CallOhlc(double[] open, double[] high, double[] low, double[] close, Func<IntPtr, IntPtr, IntPtr, IntPtr, int, IntPtr, int> invoke)
    {
        var output = Allocate(close.Length);
        fixed (double* pO = open)
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pOut = output)
        {
            int code = invoke((IntPtr)pO, (IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"indicator failed with error code {code}");
        }
        return output;
    }

    /// <summary>
    /// Runs a four-price candlestick detector and returns the integer pattern
    /// codes (+100 / 0 / -100), matching TA-Lib's convention.
    /// </summary>
    private static unsafe int[] CallCdl(double[] open, double[] high, double[] low, double[] close, Func<IntPtr, IntPtr, IntPtr, IntPtr, int, IntPtr, int> invoke)
    {
        var output = AllocateInt(close.Length);
        fixed (double* pO = open)
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (int* pOut = output)
        {
            int code = invoke((IntPtr)pO, (IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"pattern detector failed with error code {code}");
        }
        return output;
    }

    private static unsafe int[] CallCdlPct(double[] open, double[] high, double[] low, double[] close, double pct, Func<IntPtr, IntPtr, IntPtr, IntPtr, int, double, IntPtr, int> invoke)
    {
        var output = AllocateInt(close.Length);
        fixed (double* pO = open)
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (int* pOut = output)
        {
            int code = invoke((IntPtr)pO, (IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, pct, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"pattern detector failed with error code {code}");
        }
        return output;
    }

    // ========================================================================
    // Momentum / trend
    // ========================================================================

    /// <summary>Absolute Price Oscillator.</summary>
    public static double[] Apo(double[] input, int fastPeriod = 12, int slowPeriod = 26) =>
        Call1(input, (pIn, len, pOut) => ta_apo(pIn, len, fastPeriod, slowPeriod, pOut));

    /// <summary>Chande Momentum Oscillator.</summary>
    public static double[] Cmo(double[] input, int period = 14) =>
        Call1(input, (pIn, len, pOut) => ta_cmo(pIn, len, period, pOut));

    /// <summary>1-day rate of change of a triple-smoothed EMA.</summary>
    public static double[] Trix(double[] input, int period = 15) =>
        Call1(input, (pIn, len, pOut) => ta_trix(pIn, len, period, pOut));

    /// <summary>Percentile rank of the current value within the window.</summary>
    public static double[] PercentRank(double[] input, int period = 14) =>
        Call1(input, (pIn, len, pOut) => ta_percent_rank(pIn, len, period, pOut));

    /// <summary>Chande Forecast Oscillator.</summary>
    public static double[] ChandeForecast(double[] close, int period = 14) =>
        Call1(close, (pIn, len, pOut) => ta_chande_forecast(pIn, len, period, pOut));

    /// <summary>Inertia: an RVI-smoothed linear-regression trend measure.</summary>
    public static double[] Inertia(double[] open, double[] high, double[] low, double[] close, int rviPeriod = 14, int linregPeriod = 20) =>
        CallOhlc(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_inertia(pO, pH, pL, pC, len, rviPeriod, linregPeriod, pOut));

    /// <summary>Parabolic SAR.</summary>
    public static unsafe double[] Sar(double[] high, double[] low, double acceleration = 0.02, double maximum = 0.2)
    {
        var output = Allocate(high.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pOut = output)
        {
            int code = ta_sar((IntPtr)pH, (IntPtr)pL, high.Length, acceleration, maximum, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"SAR failed with error code {code}");
        }
        return output;
    }

    // ========================================================================
    // Price transforms
    // ========================================================================

    /// <summary>(open + high + low + close) / 4.</summary>
    public static double[] AvgPrice(double[] open, double[] high, double[] low, double[] close) =>
        CallOhlc(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_avgprice(pO, pH, pL, pC, len, pOut));

    /// <summary>(high + low) / 2.</summary>
    public static unsafe double[] MedPrice(double[] high, double[] low)
    {
        var output = Allocate(high.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pOut = output)
        {
            int code = ta_medprice((IntPtr)pH, (IntPtr)pL, high.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"MEDPRICE failed with error code {code}");
        }
        return output;
    }

    /// <summary>(high + low + close) / 3.</summary>
    public static unsafe double[] TypPrice(double[] high, double[] low, double[] close)
    {
        var output = Allocate(close.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pOut = output)
        {
            int code = ta_typprice((IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"TYPPRICE failed with error code {code}");
        }
        return output;
    }

    /// <summary>(high + low + 2 * close) / 4.</summary>
    public static unsafe double[] WclPrice(double[] high, double[] low, double[] close)
    {
        var output = Allocate(close.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pOut = output)
        {
            int code = ta_wclprice((IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"WCLPRICE failed with error code {code}");
        }
        return output;
    }

    /// <summary>Rolling midpoint of the input.</summary>
    public static double[] MidPoint(double[] input, int period = 14) =>
        Call1(input, (pIn, len, pOut) => ta_midpoint(pIn, len, period, pOut));

    /// <summary>Rolling midpoint of (high, low).</summary>
    public static unsafe double[] MidPrice(double[] high, double[] low, int period = 14)
    {
        var output = Allocate(high.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pOut = output)
        {
            int code = ta_midprice((IntPtr)pH, (IntPtr)pL, high.Length, period, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"MIDPRICE failed with error code {code}");
        }
        return output;
    }

    // ========================================================================
    // Volume
    // ========================================================================

    /// <summary>Balance of Power.</summary>
    public static double[] Bop(double[] open, double[] high, double[] low, double[] close) =>
        CallOhlc(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_bop(pO, pH, pL, pC, len, pOut));

    /// <summary>Money Flow Index.</summary>
    public static unsafe double[] Mfi(double[] high, double[] low, double[] close, double[] volume, int period = 14)
    {
        var output = Allocate(close.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pV = volume)
        fixed (double* pOut = output)
        {
            int code = ta_mfi((IntPtr)pH, (IntPtr)pL, (IntPtr)pC, (IntPtr)pV, close.Length, period, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"MFI failed with error code {code}");
        }
        return output;
    }

    /// <summary>Volume Zone Oscillator.</summary>
    public static unsafe double[] Vzo(double[] close, double[] volume, int period = 14)
    {
        var output = Allocate(close.Length);
        fixed (double* pC = close)
        fixed (double* pV = volume)
        fixed (double* pOut = output)
        {
            int code = ta_vzo((IntPtr)pC, (IntPtr)pV, close.Length, period, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"VZO failed with error code {code}");
        }
        return output;
    }

    /// <summary>Volume momentum oscillator.</summary>
    public static double[] VolumeMomentum(double[] volume, int period = 14) =>
        Call1(volume, (pIn, len, pOut) => ta_volume_momentum(pIn, len, period, pOut));

    /// <summary>Volume rate of change.</summary>
    public static double[] VolumeRoc(double[] volume, int period = 14) =>
        Call1(volume, (pIn, len, pOut) => ta_volume_roc(pIn, len, period, pOut));

    /// <summary>Twiggs Money Flow.</summary>
    public static unsafe double[] TwiggsMoneyFlow(double[] high, double[] low, double[] close, double[] volume, int period = 21)
    {
        var output = Allocate(close.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pV = volume)
        fixed (double* pOut = output)
        {
            int code = ta_twiggs_mf((IntPtr)pH, (IntPtr)pL, (IntPtr)pC, (IntPtr)pV, close.Length, period, (IntPtr)pOut);
            if (code != 0) throw new InvalidOperationException($"Twiggs MF failed with error code {code}");
        }
        return output;
    }

    /// <summary>Vortex Indicator: the two lines VI+ and VI-.</summary>
    public static unsafe VortexResult Vortex(double[] high, double[] low, double[] close, int period = 14)
    {
        var plus = Allocate(close.Length);
        var minus = Allocate(close.Length);
        fixed (double* pH = high)
        fixed (double* pL = low)
        fixed (double* pC = close)
        fixed (double* pPlus = plus)
        fixed (double* pMinus = minus)
        {
            int code = ta_vortex((IntPtr)pH, (IntPtr)pL, (IntPtr)pC, close.Length, period, (IntPtr)pPlus, (IntPtr)pMinus);
            if (code != 0) throw new InvalidOperationException($"Vortex failed with error code {code}");
        }
        return new VortexResult(plus, minus);
    }

    // ========================================================================
    // Candlestick patterns
    // ========================================================================

    /// <summary>Doji. Values are +100 / 0 / -100.</summary>
    public static int[] CdlDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct = 0.1) =>
        CallCdlPct(open, high, low, close, dojiPct, (pO, pH, pL, pC, len, pct, pOut) => ta_cdl_doji(pO, pH, pL, pC, len, pct, pOut));

    /// <summary>Dragonfly Doji.</summary>
    public static int[] CdlDragonflyDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct = 0.1) =>
        CallCdlPct(open, high, low, close, dojiPct, (pO, pH, pL, pC, len, pct, pOut) => ta_cdl_dragonfly_doji(pO, pH, pL, pC, len, pct, pOut));

    /// <summary>Gravestone Doji.</summary>
    public static int[] CdlGravestoneDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct = 0.1) =>
        CallCdlPct(open, high, low, close, dojiPct, (pO, pH, pL, pC, len, pct, pOut) => ta_cdl_gravestone_doji(pO, pH, pL, pC, len, pct, pOut));

    /// <summary>Long-Legged Doji.</summary>
    public static int[] CdlLongLeggedDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct = 0.1) =>
        CallCdlPct(open, high, low, close, dojiPct, (pO, pH, pL, pC, len, pct, pOut) => ta_cdl_long_legged_doji(pO, pH, pL, pC, len, pct, pOut));

    /// <summary>Marubozu.</summary>
    public static int[] CdlMarubozu(double[] open, double[] high, double[] low, double[] close, double shadowPct = 0.1) =>
        CallCdlPct(open, high, low, close, shadowPct, (pO, pH, pL, pC, len, pct, pOut) => ta_cdl_marubozu(pO, pH, pL, pC, len, pct, pOut));

    /// <summary>Hammer.</summary>
    public static int[] CdlHammer(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_hammer(pO, pH, pL, pC, len, pOut));

    /// <summary>Inverted Hammer.</summary>
    public static int[] CdlInvertedHammer(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_inverted_hammer(pO, pH, pL, pC, len, pOut));

    /// <summary>Hanging Man.</summary>
    public static int[] CdlHangingMan(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_hanging_man(pO, pH, pL, pC, len, pOut));

    /// <summary>Shooting Star.</summary>
    public static int[] CdlShootingStar(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_shooting_star(pO, pH, pL, pC, len, pOut));

    /// <summary>Engulfing.</summary>
    public static int[] CdlEngulfing(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_engulfing(pO, pH, pL, pC, len, pOut));

    /// <summary>Harami.</summary>
    public static int[] CdlHarami(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_harami(pO, pH, pL, pC, len, pOut));

    /// <summary>Morning Star.</summary>
    public static int[] CdlMorningStar(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_morning_star(pO, pH, pL, pC, len, pOut));

    /// <summary>Evening Star.</summary>
    public static int[] CdlEveningStar(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_evening_star(pO, pH, pL, pC, len, pOut));

    /// <summary>Three White Soldiers.</summary>
    public static int[] CdlThreeWhiteSoldiers(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_three_white_soldiers(pO, pH, pL, pC, len, pOut));

    /// <summary>Three Black Crows.</summary>
    public static int[] CdlThreeBlackCrows(double[] open, double[] high, double[] low, double[] close) =>
        CallCdl(open, high, low, close, (pO, pH, pL, pC, len, pOut) => ta_cdl_three_black_crows(pO, pH, pL, pC, len, pOut));
}

/// <summary>Two-line result of <see cref="ParityIndicators.Vortex"/>.</summary>
public class VortexResult
{
    /// <summary>Creates a Vortex result from its two lines.</summary>
    public VortexResult(double[] viPlus, double[] viMinus)
    {
        ViPlus = viPlus;
        ViMinus = viMinus;
    }

    /// <summary>Positive Vortex line (VI+).</summary>
    public double[] ViPlus { get; }

    /// <summary>Negative Vortex line (VI-).</summary>
    public double[] ViMinus { get; }
}
