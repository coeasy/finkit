using Xunit;

namespace AlphaTA.Tests;

public class IndicatorTests
{
    [Fact]
    public void Sma_ReturnsCorrectLength()
    {
        var input = Enumerable.Range(1, 20).Select(i => (double)i).ToArray();
        var result = Indicators.Sma(input, 5);
        Assert.Equal(input.Length, result.Length);
    }

    [Fact]
    public void Sma_CalculatesCorrectValue()
    {
        var input = new double[] { 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0 };
        var result = Indicators.Sma(input, 3);

        // SMA(3) of [1,2,3] = (1+2+3)/3 = 2.0
        Assert.Equal(2.0, result[2], 10);
        // SMA(3) of [2,3,4] = (2+3+4)/3 = 3.0
        Assert.Equal(3.0, result[3], 10);
        // SMA(3) of [8,9,10] = (8+9+10)/3 = 9.0
        Assert.Equal(9.0, result[9], 10);
    }

    [Fact]
    public void Ema_ReturnsCorrectLength()
    {
        var input = Enumerable.Range(1, 20).Select(i => (double)i).ToArray();
        var result = Indicators.Ema(input, 5);
        Assert.Equal(input.Length, result.Length);
    }

    [Fact]
    public void Rsi_ReturnsCorrectRange()
    {
        var input = new double[] { 44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5, 45.25, 46.0, 46.25, 47.0, 47.0, 47.25, 48.25 };
        var result = Indicators.Rsi(input, 14);

        // RSI should be between 0 and 100
        for (int i = 14; i < result.Length; i++)
        {
            Assert.True(result[i] >= 0 && result[i] <= 100, $"RSI value at index {i} is {result[i]}, expected 0-100");
        }
    }

    [Fact]
    public void Macd_ReturnsCorrectLengths()
    {
        var input = Enumerable.Range(1, 50).Select(i => (double)i).ToArray();
        var result = Indicators.Macd(input, 12, 26, 9);

        Assert.Equal(input.Length, result.Macd.Length);
        Assert.Equal(input.Length, result.Signal.Length);
        Assert.Equal(input.Length, result.Hist.Length);
    }

    [Fact]
    public void Bbands_ReturnsCorrectStructure()
    {
        var input = Enumerable.Range(1, 30).Select(i => (double)i).ToArray();
        var result = Indicators.Bbands(input, 10, 2.0, 2.0);

        Assert.Equal(input.Length, result.Upper.Length);
        Assert.Equal(input.Length, result.Middle.Length);
        Assert.Equal(input.Length, result.Lower.Length);

        // Upper should be > Middle should be > Lower
        for (int i = 9; i < result.Upper.Length; i++)
        {
            if (!double.IsNaN(result.Upper[i]) && !double.IsNaN(result.Middle[i]) && !double.IsNaN(result.Lower[i]))
            {
                Assert.True(result.Upper[i] >= result.Middle[i], $"Upper[{i}] should be >= Middle[{i}]");
                Assert.True(result.Middle[i] >= result.Lower[i], $"Middle[{i}] should be >= Lower[{i}]");
            }
        }
    }

    [Fact]
    public void Stoch_ReturnsCorrectStructure()
    {
        var high = new double[] { 10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0 };
        var low = new double[] { 8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0 };
        var close = new double[] { 9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0 };

        var result = Indicators.Stoch(high, low, close, 5, 3, 3);

        Assert.Equal(high.Length, result.K.Length);
        Assert.Equal(high.Length, result.D.Length);
    }

    [Fact]
    public void Atr_ReturnsCorrectLength()
    {
        var high = Enumerable.Range(1, 20).Select(i => (double)i + 1.0).ToArray();
        var low = Enumerable.Range(1, 20).Select(i => (double)i - 1.0).ToArray();
        var close = Enumerable.Range(1, 20).Select(i => (double)i).ToArray();

        var result = Indicators.Atr(high, low, close, 14);

        Assert.Equal(high.Length, result.Length);
    }

    [Fact]
    public void HtDcPeriod_ReturnsCorrectLength()
    {
        var input = Enumerable.Range(0, 60).Select(i => Math.Sin(i * 0.2) * 10.0 + 50.0).ToArray();
        var result = Indicators.HtDcPeriod(input);

        Assert.Equal(input.Length, result.Length);
    }

    [Fact]
    public void Mama_ReturnsCorrectStructure()
    {
        var input = Enumerable.Range(0, 50).Select(i => Math.Sin(i * 0.2)).ToArray();
        var result = Indicators.Mama(input, 0.5, 0.05);

        Assert.Equal(input.Length, result.Mama.Length);
        Assert.Equal(input.Length, result.Fama.Length);
    }

    [Fact]
    public void Correlation_ReturnsCorrectRange()
    {
        var inputA = new double[] { 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0 };
        var inputB = new double[] { 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0 };

        var result = Indicators.Correlation(inputA, inputB, 5);

        // For perfectly correlated data, correlation should be close to 1.0
        for (int i = 4; i < result.Length; i++)
        {
            if (!double.IsNaN(result[i]))
            {
                Assert.True(result[i] >= -1.0 && result[i] <= 1.0, $"Correlation[{i}] = {result[i]}, expected -1 to 1");
                Assert.True(result[i] > 0.99, $"Expected near 1.0 for perfectly correlated data, got {result[i]}");
            }
        }
    }

    [Fact]
    public void Obv_ReturnsCorrectLength()
    {
        var close = new double[] { 10.0, 11.0, 10.0, 12.0, 13.0 };
        var volume = new double[] { 100.0, 200.0, 150.0, 300.0, 250.0 };

        var result = Indicators.Obv(close, volume);

        Assert.Equal(close.Length, result.Length);
    }

    [Fact]
    public void LinearReg_ReturnsCorrectLength()
    {
        var input = Enumerable.Range(1, 10).Select(i => (double)i).ToArray();
        var result = Indicators.LinearReg(input, 5);

        Assert.Equal(input.Length, result.Length);
    }

    [Fact]
    public void Adx_ReturnsNonNegativeValues()
    {
        var high = new double[] { 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0, 32.0, 33.0 };
        var low = new double[] { 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0 };
        var close = new double[] { 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0, 32.0 };

        var result = Indicators.Adx(high, low, close, 14);

        for (int i = 0; i < result.Length; i++)
        {
            if (!double.IsNaN(result[i]))
            {
                Assert.True(result[i] >= 0, $"ADX[{i}] = {result[i]}, expected >= 0");
            }
        }
    }

    [Fact]
    public void ZScore_ProducesValidValues()
    {
        var input = new double[] { 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0 };
        var result = Indicators.ZScore(input, 5);

        // Initial values should be NaN
        for (int i = 0; i < 4; i++)
        {
            Assert.True(double.IsNaN(result[i]), $"ZScore[{i}] should be NaN");
        }

        // Later values should be valid
        Assert.False(double.IsNaN(result[4]), "ZScore[4] should not be NaN");
        Assert.False(double.IsNaN(result[9]), "ZScore[9] should not be NaN");
    }

    [Fact]
    public void HtTrendMode_ReturnsBinaryValues()
    {
        var input = Enumerable.Range(0, 60).Select(i => (double)i).ToArray();
        var result = Indicators.HtTrendMode(input);

        for (int i = 0; i < result.Length; i++)
        {
            if (!double.IsNaN(result[i]))
            {
                Assert.True(result[i] == 0.0 || result[i] == 1.0, $"HT_TRENDMODE[{i}] = {result[i]}, expected 0 or 1");
            }
        }
    }

    [Fact]
    public void StdDev_ProducesNonNegativeValues()
    {
        var input = new double[] { 2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0 };
        var result = Indicators.StdDev(input, 5, 1.0);

        for (int i = 0; i < result.Length; i++)
        {
            if (!double.IsNaN(result[i]))
            {
                Assert.True(result[i] >= 0, $"StdDev[{i}] = {result[i]}, expected >= 0");
            }
        }
    }

    [Fact]
    public void Natr_ReturnsPercentageValues()
    {
        var high = new double[] { 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0 };
        var low = new double[] { 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0 };
        var close = new double[] { 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0 };

        var result = Indicators.Natr(high, low, close, 14);

        for (int i = 0; i < result.Length; i++)
        {
            if (!double.IsNaN(result[i]))
            {
                Assert.True(result[i] >= 0, $"NATR[{i}] = {result[i]}, expected >= 0");
            }
        }
    }
}
