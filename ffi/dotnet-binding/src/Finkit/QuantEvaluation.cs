using System;
using System.Runtime.InteropServices;
namespace Finkit;
/// <summary>Canonical quantitative evaluation backed by Rust.</summary>
public static class QuantEvaluation
{
    private const string NativeLibrary = "finkit_dotnet";
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr finkit_dotnet_quant_evaluation_json([MarshalAs(UnmanagedType.LPUTF8Str)] string requestJson);
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern void finkit_dotnet_factor_study_free_string(IntPtr value);

    /// <summary>Evaluates a versioned quantitative-performance request through the canonical Rust engine.</summary>
    /// <param name="requestJson">UTF-8 JSON request containing returns and optional benchmark, trade, portfolio, and cost inputs.</param>
    /// <returns>The UTF-8 JSON quantitative-evaluation response produced by the native engine.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="requestJson"/> is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the native engine unexpectedly returns a null pointer.</exception>
    public static string RunJson(string requestJson)
    {
        ArgumentNullException.ThrowIfNull(requestJson);
        var ptr = finkit_dotnet_quant_evaluation_json(requestJson);
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("Native quantitative evaluation returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { finkit_dotnet_factor_study_free_string(ptr); }
    }
}