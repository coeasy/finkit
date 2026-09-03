#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

cmake = ROOT / "ffi/c-binding/CMakeLists.txt"
text = cmake.read_text(encoding="utf-8")
text = text.replace("find_package(PkgConfig REQUIRED)\n", "")
cmake.write_text(text, encoding="utf-8")

loader = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/NativeLoader.java"
text = loader.read_text(encoding="utf-8")
if "import java.nio.file.Paths;" not in text:
    text = text.replace(
        "import java.nio.file.Path;\n",
        "import java.nio.file.Path;\nimport java.nio.file.Paths;\n",
        1,
    )
text = text.replace(
    "System.load(Path.of(explicitPath).toAbsolutePath().toString());",
    "System.load(Paths.get(explicitPath).toAbsolutePath().toString());",
)
loader.write_text(text, encoding="utf-8")
