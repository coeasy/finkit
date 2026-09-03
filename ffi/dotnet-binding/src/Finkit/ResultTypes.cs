namespace Finkit;

/// <summary>
/// Result from MACD (Moving Average Convergence Divergence) calculation.
/// </summary>
public class MacdResult
{
    /// <summary>MACD line values.</summary>
    public double[] Macd { get; }

    /// <summary>Signal line values (EMA of MACD).</summary>
    public double[] Signal { get; }

    /// <summary>Histogram values (MACD - Signal).</summary>
    public double[] Hist { get; }

    /// <summary>Creates a new MACD result.</summary>
    /// <param name="macd">MACD line array.</param>
    /// <param name="signal">Signal line array.</param>
    /// <param name="hist">Histogram array.</param>
    public MacdResult(double[] macd, double[] signal, double[] hist)
    {
        Macd = macd;
        Signal = signal;
        Hist = hist;
    }
}

/// <summary>
/// Result from Bollinger Bands (BBANDS) calculation.
/// </summary>
public class BbandsResult
{
    /// <summary>Upper band values.</summary>
    public double[] Upper { get; }

    /// <summary>Middle band values (SMA).</summary>
    public double[] Middle { get; }

    /// <summary>Lower band values.</summary>
    public double[] Lower { get; }

    /// <summary>Creates a new Bollinger Bands result.</summary>
    /// <param name="upper">Upper band array.</param>
    /// <param name="middle">Middle band array.</param>
    /// <param name="lower">Lower band array.</param>
    public BbandsResult(double[] upper, double[] middle, double[] lower)
    {
        Upper = upper;
        Middle = middle;
        Lower = lower;
    }
}

/// <summary>
/// Result from Stochastic Oscillator (STOCH) calculation.
/// </summary>
public class StochResult
{
    /// <summary>%K line values (fast).</summary>
    public double[] K { get; }

    /// <summary>%D line values (slow, SMA of %K).</summary>
    public double[] D { get; }

    /// <summary>Creates a new Stochastic result.</summary>
    /// <param name="k">%K array.</param>
    /// <param name="d">%D array.</param>
    public StochResult(double[] k, double[] d)
    {
        K = k;
        D = d;
    }
}

/// <summary>
/// Result from Aroon Indicator calculation.
/// </summary>
public class AroonResult
{
    /// <summary>Aroon Up values.</summary>
    public double[] AroonUp { get; }

    /// <summary>Aroon Down values.</summary>
    public double[] AroonDown { get; }

    /// <summary>Creates a new Aroon result.</summary>
    /// <param name="aroonUp">Aroon Up array.</param>
    /// <param name="aroonDown">Aroon Down array.</param>
    public AroonResult(double[] aroonUp, double[] aroonDown)
    {
        AroonUp = aroonUp;
        AroonDown = aroonDown;
    }
}

/// <summary>
/// Result from MAMA (MESA Adaptive Moving Average) calculation.
/// </summary>
public class MamaResult
{
    /// <summary>MAMA values.</summary>
    public double[] Mama { get; }

    /// <summary>FAMA (Following Adaptive Moving Average) values.</summary>
    public double[] Fama { get; }

    /// <summary>Creates a new MAMA result.</summary>
    /// <param name="mama">MAMA array.</param>
    /// <param name="fama">FAMA array.</param>
    public MamaResult(double[] mama, double[] fama)
    {
        Mama = mama;
        Fama = fama;
    }
}

/// <summary>
/// Result from Hilbert Phasor calculation.
/// </summary>
public class HtPhasorResult
{
    /// <summary>In-phase component values.</summary>
    public double[] InPhase { get; }

    /// <summary>Quadrature component values.</summary>
    public double[] Quadrature { get; }

    /// <summary>Creates a new Hilbert Phasor result.</summary>
    /// <param name="inPhase">In-phase array.</param>
    /// <param name="quadrature">Quadrature array.</param>
    public HtPhasorResult(double[] inPhase, double[] quadrature)
    {
        InPhase = inPhase;
        Quadrature = quadrature;
    }
}

/// <summary>
/// Result from Hilbert Sine Wave calculation.
/// </summary>
public class HtSineResult
{
    /// <summary>Sine wave values.</summary>
    public double[] Sine { get; }

    /// <summary>Lead sine wave values (phase-shifted by 45 degrees).</summary>
    public double[] LeadSine { get; }

    /// <summary>Creates a new Hilbert Sine result.</summary>
    /// <param name="sine">Sine array.</param>
    /// <param name="leadSine">Lead sine array.</param>
    public HtSineResult(double[] sine, double[] leadSine)
    {
        Sine = sine;
        LeadSine = leadSine;
    }
}

/// <summary>Result from Darvas box calculation.</summary>
public class DarvasBoxResult
{
    /// <summary>Upper boundary of the active Darvas box for each bar.</summary>
    public double[] BoxTop { get; }

    /// <summary>Lower boundary of the active Darvas box for each bar.</summary>
    public double[] BoxBottom { get; }

    /// <summary>Per-bar breakout signal emitted by the Darvas box calculation.</summary>
    public int[] Signal { get; }

    /// <summary>Creates a Darvas box result.</summary>
    /// <param name="boxTop">Upper box boundary values.</param>
    /// <param name="boxBottom">Lower box boundary values.</param>
    /// <param name="signal">Per-bar breakout signals.</param>
    public DarvasBoxResult(double[] boxTop, double[] boxBottom, int[] signal)
    {
        BoxTop = boxTop;
        BoxBottom = boxBottom;
        Signal = signal;
    }
}

/// <summary>Result from Renko calculation.</summary>
public class RenkoResult
{
    /// <summary>Renko brick values aligned to the input series.</summary>
    public double[] Bricks { get; }

    /// <summary>Per-bar brick direction values.</summary>
    public int[] Direction { get; }

    /// <summary>Creates a Renko result.</summary>
    /// <param name="bricks">Renko brick values.</param>
    /// <param name="direction">Per-bar direction values.</param>
    public RenkoResult(double[] bricks, int[] direction)
    {
        Bricks = bricks;
        Direction = direction;
    }
}

/// <summary>Result from Kagi calculation.</summary>
public class KagiResult
{
    /// <summary>Kagi line values aligned to the input series.</summary>
    public double[] Values { get; }

    /// <summary>Per-bar Kagi direction values.</summary>
    public int[] Direction { get; }

    /// <summary>Creates a Kagi result.</summary>
    /// <param name="values">Kagi line values.</param>
    /// <param name="direction">Per-bar direction values.</param>
    public KagiResult(double[] values, int[] direction)
    {
        Values = values;
        Direction = direction;
    }
}

/// <summary>Result from point-and-figure calculation.</summary>
public class PointAndFigureResult
{
    /// <summary>Point-and-figure values aligned to the input series.</summary>
    public double[] Values { get; }

    /// <summary>Column direction/state values.</summary>
    public int[] Column { get; }

    /// <summary>Flags indicating where a new point-and-figure column begins.</summary>
    public int[] NewColumn { get; }

    /// <summary>Creates a point-and-figure result.</summary>
    /// <param name="values">Point-and-figure values.</param>
    /// <param name="column">Column state values.</param>
    /// <param name="newColumn">New-column flags.</param>
    public PointAndFigureResult(double[] values, int[] column, int[] newColumn)
    {
        Values = values;
        Column = column;
        NewColumn = newColumn;
    }
}

/// <summary>Result from three-line-break calculation.</summary>
public class ThreeLineBreakResult
{
    /// <summary>Three-line-break values aligned to the input series.</summary>
    public double[] Values { get; }

    /// <summary>Per-bar three-line-break direction values.</summary>
    public int[] Direction { get; }

    /// <summary>Creates a three-line-break result.</summary>
    /// <param name="values">Three-line-break values.</param>
    /// <param name="direction">Per-bar direction values.</param>
    public ThreeLineBreakResult(double[] values, int[] direction)
    {
        Values = values;
        Direction = direction;
    }
}

/// <summary>Result from Williams Alligator calculation.</summary>
public class WilliamsAlligatorResult
{
    /// <summary>Jaw line values.</summary>
    public double[] Jaw { get; }

    /// <summary>Teeth line values.</summary>
    public double[] Teeth { get; }

    /// <summary>Lips line values.</summary>
    public double[] Lips { get; }

    /// <summary>Creates a Williams Alligator result.</summary>
    /// <param name="jaw">Jaw line values.</param>
    /// <param name="teeth">Teeth line values.</param>
    /// <param name="lips">Lips line values.</param>
    public WilliamsAlligatorResult(double[] jaw, double[] teeth, double[] lips)
    {
        Jaw = jaw;
        Teeth = teeth;
        Lips = lips;
    }
}

/// <summary>Result from Heikin-Ashi calculation.</summary>
public class HeikinAshiResult
{
    /// <summary>Heikin-Ashi open values.</summary>
    public double[] Open { get; }

    /// <summary>Heikin-Ashi high values.</summary>
    public double[] High { get; }

    /// <summary>Heikin-Ashi low values.</summary>
    public double[] Low { get; }

    /// <summary>Heikin-Ashi close values.</summary>
    public double[] Close { get; }

    /// <summary>Creates a Heikin-Ashi result.</summary>
    /// <param name="open">Heikin-Ashi open values.</param>
    /// <param name="high">Heikin-Ashi high values.</param>
    /// <param name="low">Heikin-Ashi low values.</param>
    /// <param name="close">Heikin-Ashi close values.</param>
    public HeikinAshiResult(double[] open, double[] high, double[] low, double[] close)
    {
        Open = open;
        High = high;
        Low = low;
        Close = close;
    }
}
