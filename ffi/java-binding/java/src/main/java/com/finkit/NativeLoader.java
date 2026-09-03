package com.finkit;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/** Loads the platform JNI library from the packaged JAR with a system-library fallback. */
final class NativeLoader {
    private static final String LIBRARY_NAME = "finkit_java";
    private static volatile boolean loaded;

    private NativeLoader() {
    }

    static synchronized void load() {
        if (loaded) {
            return;
        }

        String explicitPath = System.getProperty("finkit.native.path", "").trim();
        if (!explicitPath.isEmpty()) {
            System.load(Paths.get(explicitPath).toAbsolutePath().toString());
            loaded = true;
            return;
        }

        UnsatisfiedLinkError embeddedFailure = null;
        String mappedName = System.mapLibraryName(LIBRARY_NAME);
        String resourcePath = "/natives/" + platformKey() + "/" + mappedName;

        try (InputStream input = NativeLoader.class.getResourceAsStream(resourcePath)) {
            if (input != null) {
                Path directory = Files.createTempDirectory("finkit-native-");
                Path library = directory.resolve(mappedName);
                Files.copy(input, library, StandardCopyOption.REPLACE_EXISTING);
                directory.toFile().deleteOnExit();
                library.toFile().deleteOnExit();
                System.load(library.toAbsolutePath().toString());
                loaded = true;
                return;
            }
        } catch (IOException | UnsatisfiedLinkError error) {
            embeddedFailure = new UnsatisfiedLinkError(
                    "Failed to load embedded Finkit native library " + resourcePath + ": " + error.getMessage());
            embeddedFailure.initCause(error);
        }

        try {
            System.loadLibrary(LIBRARY_NAME);
            loaded = true;
        } catch (UnsatisfiedLinkError error) {
            if (embeddedFailure != null) {
                error.addSuppressed(embeddedFailure);
            }
            throw error;
        }
    }

    private static String platformKey() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);

        String normalizedOs;
        if (os.contains("win")) {
            normalizedOs = "windows";
        } else if (os.contains("mac") || os.contains("darwin")) {
            normalizedOs = "macos";
        } else if (os.contains("nix") || os.contains("nux") || os.contains("aix") || os.contains("linux")) {
            normalizedOs = "linux";
        } else {
            throw new UnsatisfiedLinkError("Unsupported operating system: " + os);
        }

        String normalizedArch;
        if (arch.equals("amd64") || arch.equals("x86_64") || arch.equals("x64")) {
            normalizedArch = "x86_64";
        } else if (arch.equals("aarch64") || arch.equals("arm64")) {
            normalizedArch = "aarch64";
        } else {
            throw new UnsatisfiedLinkError("Unsupported architecture: " + arch);
        }

        return normalizedOs + "-" + normalizedArch;
    }
}
