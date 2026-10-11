fn main() {
    // ggml's Metal backend uses Objective-C `@available`, which clang lowers to
    // `___isPlatformVersionAtLeast` from compiler-rt; rustc's link line lacks it. The env vars
    // (not `cfg!`) describe the target, since a build script runs on the host.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var_os("CARGO_FEATURE_EMBEDDED_AI").is_some()
    {
        link_clang_rt();
    }
    tauri_build::build()
}

/// Links clang's `libclang_rt.osx.a` (a fat archive: one search path serves both arches).
fn link_clang_rt() {
    let out = std::process::Command::new("cc").arg("-print-resource-dir").output();
    let Ok(out) = out.map_err(|e| println!("cargo:warning=cc -print-resource-dir: {e}")) else {
        return;
    };
    let dir = String::from_utf8_lossy(&out.stdout);
    println!("cargo:rustc-link-search=native={}/lib/darwin", dir.trim());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}
