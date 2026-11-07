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
        // TA-Lib 0.8.x emits `libta-lib.a` on Unix but keeps the
        // `ta-lib-static.lib` name on Windows. Keep the crate link name in
        // sync with the upstream CMake target's platform-specific output.
        let static_lib_name = if cfg!(windows) {
            "ta-lib-static"
        } else {
            "ta-lib"
        };

        println!("cargo:rustc-link-search=native={ta_lib_dir}/lib");
        println!("cargo:rustc-link-lib=static={static_lib_name}");

        // Re-link if the static lib changes (e.g., when the stub is rebuilt).
        let lib_path = std::path::Path::new(&ta_lib_dir)
            .join("lib")
            .join(if cfg!(windows) {
                "ta-lib-static.lib"
            } else {
                "libta-lib.a"
            });
        if lib_path.exists() {
            println!("cargo:rerun-if-changed={}", lib_path.display());
        }

        println!("cargo:rerun-if-env-changed=TA_LIB_DIR");
    }
}
