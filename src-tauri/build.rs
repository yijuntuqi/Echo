use std::env;
use std::path::PathBuf;

fn main() {
    // 嵌入版本信息
    let version = env::var("CARGO_PKG_VERSION").unwrap_or("dev".to_string());
    println!("cargo:rustc-env=ECHO_VERSION={}", version);
    
    // 嵌入构建时间
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=ECHO_BUILD_TIME={}", build_time);
    
    // Android 目标链接预编译 sqlite-vec
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "android" {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or("arm64-v8a".to_string());
        let prebuilt_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("prebuilt")
            .join("android")
            .join(&arch)
            .join("lib");
        
        println!("cargo:rustc-link-search=native={}", prebuilt_path.display());
        println!("cargo:rustc-link-lib=sqlite3");
        println!("cargo:rustc-link-lib=vec0");
        println!("cargo:rerun-if-changed=prebuilt/android/{}/lib/libvec0.so", arch);
    }
    
    tauri_build::build()
}