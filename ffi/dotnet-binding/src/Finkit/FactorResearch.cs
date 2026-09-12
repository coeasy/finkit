using System;
using System.Runtime.InteropServices;
namespace Finkit;
/// <summary>Versioned panel factor research backed by the canonical Rust engine.</summary>
public static class FactorResearch
{
    private const string NativeLibrary = "finkit_dotnet";
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr finkit_dotnet_factor_study_json([MarshalAs(UnmanagedType.LPUTF8Str)] string requestJson);
    [DllImport(NativeLibrary, CallingConvention = CallingConvention.Cdecl)]
    private static extern void finkit_dotnet_factor_study_free_string(IntPtr value);

    /// <summary>Runs a versioned factor-study request through the canonical Rust research engine.</summary>
    /// <param name="requestJson">UTF-8 JSON request matching the supported factor-study schema.</param>
    /// <returns>The UTF-8 JSON response produced by the native research engine.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="requestJson"/> is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown when the native engine unexpectedly returns a null pointer.</exception>
    public static string RunJson(string requestJson)
    {
        ArgumentNullException.ThrowIfNull(requestJson);
        var ptr = finkit_dotnet_factor_study_json(requestJson);
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("Native factor research returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { finkit_dotnet_factor_study_free_string(ptr); }
    }
}