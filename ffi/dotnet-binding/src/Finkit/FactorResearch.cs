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
    public static string RunJson(string requestJson)
    {
        ArgumentNullException.ThrowIfNull(requestJson);
        var ptr = finkit_dotnet_factor_study_json(requestJson);
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("Native factor research returned null");
        try { return Marshal.PtrToStringUTF8(ptr) ?? string.Empty; }
        finally { finkit_dotnet_factor_study_free_string(ptr); }
    }
}
