using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Finkit;

/// <summary>
/// Registers the process-wide resolver used by Finkit's P/Invoke declarations.
///
/// The package layout follows NuGet's standard runtimes/&lt;rid&gt;/native convention,
/// while source builds can still use FINKIT_NATIVE_PATH or the operating system's
/// normal native-library search path (for example LD_LIBRARY_PATH on Linux).
/// </summary>
internal static class NativeBootstrap
{
#pragma warning disable CA2255 // Library initialization is intentional: the resolver must be installed before the first P/Invoke call.
    [ModuleInitializer]
#pragma warning restore CA2255
    internal static void Initialize()
    {
        try
        {
            NativeLibrary.SetDllImportResolver(typeof(Indicators).Assembly, Resolve);
        }
        catch (InvalidOperationException)
        {
            // A host is allowed to register its own resolver first. In that case
            // keep the host resolver rather than replacing process policy.
        }
    }

    private static IntPtr Resolve(
        string libraryName,
        Assembly assembly,
        DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, "finkit_dotnet", StringComparison.Ordinal))
        {
            return IntPtr.Zero;
        }

        var fileName = GetNativeFileName();
        foreach (var candidate in EnumerateExplicitCandidates(assembly, fileName))
        {
            if (File.Exists(candidate) && NativeLibrary.TryLoad(candidate, out var handle))
            {
                return handle;
            }
        }

        // This overload uses the OS loader directly and therefore honors PATH,
        // LD_LIBRARY_PATH, DYLD_LIBRARY_PATH and other normal process settings
        // without recursively invoking this assembly resolver.
        if (NativeLibrary.TryLoad(fileName, out var systemHandle))
        {
            return systemHandle;
        }

        return IntPtr.Zero;
    }

    private static IEnumerable<string> EnumerateExplicitCandidates(
        Assembly assembly,
        string fileName)
    {
        var configured = Environment.GetEnvironmentVariable("FINKIT_NATIVE_PATH");
        if (!string.IsNullOrWhiteSpace(configured))
        {
            yield return Directory.Exists(configured)
                ? Path.Combine(configured, fileName)
                : configured;
        }

        var baseDirectory = AppContext.BaseDirectory;
        yield return Path.Combine(baseDirectory, fileName);
        yield return Path.Combine(baseDirectory, "runtimes", GetRuntimeIdentifier(), "native", fileName);

        var assemblyDirectory = Path.GetDirectoryName(assembly.Location);
        if (!string.IsNullOrWhiteSpace(assemblyDirectory))
        {
            yield return Path.Combine(assemblyDirectory, fileName);
            yield return Path.Combine(
                assemblyDirectory,
                "runtimes",
                GetRuntimeIdentifier(),
                "native",
                fileName);
        }
    }

    private static string GetNativeFileName()
    {
        if (OperatingSystem.IsWindows()) return "finkit_dotnet.dll";
        if (OperatingSystem.IsMacOS()) return "libfinkit_dotnet.dylib";
        return "libfinkit_dotnet.so";
    }

    private static string GetRuntimeIdentifier()
    {
        var os = OperatingSystem.IsWindows()
            ? "win"
            : OperatingSystem.IsMacOS()
                ? "osx"
                : OperatingSystem.IsLinux()
                    ? "linux"
                    : "unknown";

        var arch = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.X86 => "x86",
            Architecture.Arm64 => "arm64",
            Architecture.Arm => "arm",
            _ => "unknown"
        };

        return $"{os}-{arch}";
    }
}
