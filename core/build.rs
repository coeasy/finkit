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

        println!("cargo:rustc-link-search=native={ta_lib_dir}/lib");
        println!("cargo:rustc-link-lib=static=ta-lib-static");

        // Re-link if the static lib changes (e.g., when the stub is rebuilt).
        let lib_path = std::path::Path::new(&ta_lib_dir)
            .join("lib")
            .join("ta-lib-static.lib");
        if lib_path.exists() {
            println!("cargo:rerun-if-changed={}", lib_path.display());
        }

        println!("cargo:rerun-if-env-changed=TA_LIB_DIR");
    }
}
