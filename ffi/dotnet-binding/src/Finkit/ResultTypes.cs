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
