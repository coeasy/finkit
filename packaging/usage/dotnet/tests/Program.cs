using Finkit;

var result = Indicators.Sma(new[] { 1d, 2d, 3d, 4d }, 3);
if (result.Length != 4 || !double.IsNaN(result[0]) || !double.IsNaN(result[1])
    || result[2] != 2d || result[3] != 3d)
{
    throw new InvalidOperationException($"Unexpected SMA result: {string.Join(",", result)}");
}
Console.WriteLine("installed .NET package SMA smoke passed");
