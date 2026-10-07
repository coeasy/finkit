using System;
using System.Linq;
using Xunit;

namespace Finkit.Tests;

/// <summary>
/// Coverage for <see cref="ParityIndicators"/> — the indicators added to close
/// the .NET binding's gap against the 78-indicator FFI registry.
///
/// Beyond "it returns something", these assert the two cross-binding contracts
/// the parity work depends on:
///   1. every entry point returns an array the same length as its input
///      (or the shorter of its inputs), not a truncated warm-up slice;
///   2. candlestick detectors only ever emit TA-Lib's +100 / 0 / -100.
/// </summary>
public class ParityIndicatorTests
{
    private const int Length = 128;

    private static double[] Series(int n)
    {
        var values = new double[n];
        for (int i = 0; i < n; i++)
        {
            values[i] = 100.0 + Math.Sin(i * 0.11) * 5.0 + i * 0.05;
        }
        return values;
    }

    private static double[] High(double[] close) => close.Select(c => c + 1.5).ToArray();

    private static double[] Low(double[] close) => close.Select(c => c - 1.5).ToArray();

    private static double[] Open(double[] close) => close.Select(c => c - 0.4).ToArray();

    private static double[] Volume(int n) => Enumerable.Range(0, n).Select(i => 1000.0 + i).ToArray();

    [Fact]
    public void SingleInputIndicators_PreserveLength()
    {
        var close = Series(Length);
        var volume = Volume(Length);

        Assert.Equal(Length, ParityIndicators.Apo(close).Length);
        Assert.Equal(Length, ParityIndicators.Cmo(close).Length);
        Assert.Equal(Length, ParityIndicators.Trix(close).Length);
        Assert.Equal(Length, ParityIndicators.PercentRank(close).Length);
        Assert.Equal(Length, ParityIndicators.ChandeForecast(close).Length);
        Assert.Equal(Length, ParityIndicators.MidPoint(close).Length);
        Assert.Equal(Length, ParityIndicators.VolumeMomentum(volume).Length);
        Assert.Equal(Length, ParityIndicators.VolumeRoc(volume).Length);

        // Warm-up values are produced by the core; the point of the assertion is
        // that the tail is finite, i.e. the kernel actually ran.
        Assert.Contains(ParityIndicators.Cmo(close), v => double.IsFinite(v));
    }

    [Fact]
    public void MultiInputIndicators_PreserveLength()
    {
        var close = Series(Length);
        var high = High(close);
        var low = Low(close);
        var open = Open(close);
        var volume = Volume(Length);

        Assert.Equal(Length, ParityIndicators.AvgPrice(open, high, low, close).Length);
        Assert.Equal(Length, ParityIndicators.MedPrice(high, low).Length);
        Assert.Equal(Length, ParityIndicators.TypPrice(high, low, close).Length);
        Assert.Equal(Length, ParityIndicators.WclPrice(high, low, close).Length);
        Assert.Equal(Length, ParityIndicators.MidPrice(high, low).Length);
        Assert.Equal(Length, ParityIndicators.Bop(open, high, low, close).Length);
        Assert.Equal(Length, ParityIndicators.Sar(high, low).Length);
        Assert.Equal(Length, ParityIndicators.Mfi(high, low, close, volume).Length);
        Assert.Equal(Length, ParityIndicators.Vzo(close, volume).Length);
        Assert.Equal(Length, ParityIndicators.TwiggsMoneyFlow(high, low, close, volume).Length);
        Assert.Equal(Length, ParityIndicators.Inertia(open, high, low, close).Length);
    }

    [Fact]
    public void Vortex_ReturnsBothLines()
    {
        var close = Series(Length);
        var high = High(close);
        var low = Low(close);

        var result = ParityIndicators.Vortex(high, low, close);

        Assert.Equal(Length, result.ViPlus.Length);
        Assert.Equal(Length, result.ViMinus.Length);
        Assert.Contains(result.ViPlus, v => double.IsFinite(v));
        Assert.Contains(result.ViMinus, v => double.IsFinite(v));
    }

    [Theory]
    [InlineData("doji")]
    [InlineData("dragonfly")]
    [InlineData("gravestone")]
    [InlineData("longlegged")]
    [InlineData("marubozu")]
    [InlineData("hammer")]
    [InlineData("inverted_hammer")]
    [InlineData("hanging_man")]
    [InlineData("shooting_star")]
    [InlineData("engulfing")]
    [InlineData("harami")]
    [InlineData("morning_star")]
    [InlineData("evening_star")]
    [InlineData("three_white_soldiers")]
    [InlineData("three_black_crows")]
    public void CandlestickDetectors_EmitOnlyTalibValues(string which)
    {
        var close = Series(Length);
        var high = High(close);
        var low = Low(close);
        var open = Open(close);

        int[] result = which switch
        {
            "doji" => ParityIndicators.CdlDoji(open, high, low, close),
            "dragonfly" => ParityIndicators.CdlDragonflyDoji(open, high, low, close),
            "gravestone" => ParityIndicators.CdlGravestoneDoji(open, high, low, close),
            "longlegged" => ParityIndicators.CdlLongLeggedDoji(open, high, low, close),
            "marubozu" => ParityIndicators.CdlMarubozu(open, high, low, close),
            "hammer" => ParityIndicators.CdlHammer(open, high, low, close),
            "inverted_hammer" => ParityIndicators.CdlInvertedHammer(open, high, low, close),
            "hanging_man" => ParityIndicators.CdlHangingMan(open, high, low, close),
            "shooting_star" => ParityIndicators.CdlShootingStar(open, high, low, close),
            "engulfing" => ParityIndicators.CdlEngulfing(open, high, low, close),
            "harami" => ParityIndicators.CdlHarami(open, high, low, close),
            "morning_star" => ParityIndicators.CdlMorningStar(open, high, low, close),
            "evening_star" => ParityIndicators.CdlEveningStar(open, high, low, close),
            "three_white_soldiers" => ParityIndicators.CdlThreeWhiteSoldiers(open, high, low, close),
            "three_black_crows" => ParityIndicators.CdlThreeBlackCrows(open, high, low, close),
            _ => throw new ArgumentOutOfRangeException(nameof(which), which, "unknown detector"),
        };

        Assert.Equal(Length, result.Length);
        Assert.All(result, v => Assert.Contains(v, new[] { -100, 0, 100 }));
    }

    [Fact]
    public void InvalidPeriod_ThrowsInsteadOfReturningGarbage()
    {
        var close = Series(8);
        Assert.Throws<InvalidOperationException>(() => ParityIndicators.Cmo(close, 64));
        Assert.Throws<InvalidOperationException>(() => ParityIndicators.Apo(close, 5, 64));
    }
}
