#!/usr/bin/env python3
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def patch_cmake() -> None:
    path = ROOT / "ffi/c-binding/CMakeLists.txt"
    text = path.read_text(encoding="utf-8")
    updated, count = re.subn(
        r"(project\(finkit\s*\n\s*VERSION\s+)[0-9]+\.[0-9]+\.[0-9]+",
        r"\g<1>0.1.3",
        text,
        count=1,
    )
    if count != 1:
        raise RuntimeError("CMake project version anchor not found")
    path.write_text(updated, encoding="utf-8")


def patch_java_loader() -> None:
    indicators = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/Indicators.java"
    text = indicators.read_text(encoding="utf-8")
    start = text.find("    static {\n        loadNativeLibrary();\n    }\n")
    if start >= 0:
        end_marker = "    static void ensureLoaded() {"
        end = text.find(end_marker, start)
        if end < 0:
            raise RuntimeError("Indicators ensureLoaded anchor not found")
        replacement = "    static {\n        NativeLoader.load();\n    }\n\n"
        text = text[:start] + replacement + text[end:]
        indicators.write_text(text, encoding="utf-8")
    elif "NativeLoader.load();" not in text:
        raise RuntimeError("Indicators native loader anchor not found")

    loader = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/NativeLoader.java"
    loader.write_text(
        '''package com.finkit;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
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
            System.load(Path.of(explicitPath).toAbsolutePath().toString());
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
''',
        encoding="utf-8",
    )


def patch_version_checker() -> None:
    path = ROOT / "scripts/check_versions.py"
    text = path.read_text(encoding="utf-8")

    if 'CMAKE_PROJECT = ROOT / "ffi" / "c-binding" / "CMakeLists.txt"' not in text:
        text = text.replace(
            'JAVA_POM = ROOT / "ffi" / "java-binding" / "pom.xml"\n',
            'JAVA_POM = ROOT / "ffi" / "java-binding" / "pom.xml"\n'
            'CMAKE_PROJECT = ROOT / "ffi" / "c-binding" / "CMakeLists.txt"\n',
            1,
        )

    if "def read_cmake_project_version()" not in text:
        anchor = "def read_cargo_package_versions() -> dict[str, str]:\n"
        helper = '''def read_cmake_project_version() -> str:\n    text = CMAKE_PROJECT.read_text(encoding="utf-8")\n    match = re.search(\n        r"project\\(finkit\\s+VERSION\\s+([0-9]+\\.[0-9]+\\.[0-9]+)",\n        text,\n        re.MULTILINE,\n    )\n    if not match:\n        raise ValueError(f"CMake project version not found in {CMAKE_PROJECT}")\n    return match.group(1)\n\n\n'''
        if anchor not in text:
            raise RuntimeError("check_versions read helper anchor not found")
        text = text.replace(anchor, helper + anchor, 1)

    collect_anchor = '''    for path, tag in XML_PROJECT_VERSIONS:\n        version = read_xml_project_version(path, tag)\n        if version != canonical:\n            errors.append(f"{path.relative_to(ROOT)}: {version} != {canonical}")\n'''
    if "cmake_version = read_cmake_project_version()" not in text:
        if collect_anchor not in text:
            raise RuntimeError("check_versions collect anchor not found")
        text = text.replace(
            collect_anchor,
            collect_anchor
            + '''\n    cmake_version = read_cmake_project_version()\n    if cmake_version != canonical:\n        errors.append(f"{CMAKE_PROJECT.relative_to(ROOT)}: {cmake_version} != {canonical}")\n''',
            1,
        )

    fix_anchor = '''def fix_versions(canonical: str) -> None:\n    for path, tag in XML_PROJECT_VERSIONS:\n        replace_xml_project_version(path, tag, canonical)\n    replace_first_version(PYPROJECT, canonical)\n'''
    if "failed to update CMake version in {CMAKE_PROJECT}" not in text:
        if fix_anchor not in text:
            raise RuntimeError("check_versions fix anchor not found")
        text = text.replace(
            fix_anchor,
            '''def fix_versions(canonical: str) -> None:\n    for path, tag in XML_PROJECT_VERSIONS:\n        replace_xml_project_version(path, tag, canonical)\n\n    cmake_text = CMAKE_PROJECT.read_text(encoding="utf-8")\n    cmake_text, count = re.subn(\n        r"(project\\(finkit\\s+VERSION\\s+)[0-9]+\\.[0-9]+\\.[0-9]+",\n        rf"\\g<1>{canonical}",\n        cmake_text,\n        count=1,\n        flags=re.MULTILINE,\n    )\n    if count != 1:\n        raise ValueError(f"failed to update CMake version in {CMAKE_PROJECT}")\n    CMAKE_PROJECT.write_text(cmake_text, encoding="utf-8")\n\n    replace_first_version(PYPROJECT, canonical)\n''',
            1,
        )

    path.write_text(text, encoding="utf-8")


def main() -> None:
    patch_cmake()
    patch_java_loader()
    patch_version_checker()


if __name__ == "__main__":
    main()
