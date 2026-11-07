fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(feature = "talib-c")]
    {
        let default_dir = if cfg!(windows) {
            r"C:\TA-Lib".to_string()
        } else {
            "/usr/local".to_string()
        };
        let ta_lib_dir = std::env::var("TA_LIB_DIR").unwrap_or(default_dir);
        // TA-Lib 0.8.x release packages provide the shared library on Unix and
        // its import library on Windows. Keep the link name in sync with the
        // upstream CMake target's platform-specific output.
        println!("cargo:rustc-link-search=native={ta_lib_dir}/lib");
        println!("cargo:rustc-link-lib=dylib=ta-lib");

        // Re-link if the TA-Lib library changes (e.g., when the stub is rebuilt).
        let lib_file_name = if cfg!(windows) {
            "ta-lib.lib"
        } else if cfg!(target_os = "macos") {
            "libta-lib.dylib"
        } else {
            "libta-lib.so"
        };
        let lib_path = std::path::Path::new(&ta_lib_dir)
            .join("lib")
            .join(lib_file_name);
        if lib_path.exists() {
            println!("cargo:rerun-if-changed={}", lib_path.display());
        }

        println!("cargo:rerun-if-env-changed=TA_LIB_DIR");
    }
}
