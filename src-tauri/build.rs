fn main() {
    tauri_build::build();

    // tauri-plugin-dialog pulls in rfd with `common-controls-v6`, so linked
    // binaries import ComCtl32 v6-only APIs (e.g. TaskDialogIndirect).
    // Application binaries get the v6 dependency through the embedded
    // manifest, but test executables have no manifest and fail to start with
    // STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139) because the loader binds the
    // ComCtl32 v5 DLL, which lacks those exports. Delay-load ComCtl32 so
    // binding happens on first use instead of at process start.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
        println!("cargo:rustc-link-arg=delayimp.lib");
    }
}
