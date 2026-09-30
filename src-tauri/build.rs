fn main() {
    // Embed build metadata for `get_system_info` and bug reports.
    println!("cargo:rustc-env=ECHO_BUILD_TIME={}", chrono_like_now());

    // The pet window needs a prebuilt sqlite-vec on Android; on desktop the
    // extension is compiled in via libsqlite3-sys.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "arm64-v8a".into());
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let lib_dir = format!("{manifest}/prebuilt/android/{arch}/lib");
        println!("cargo:rustc-link-search=native={lib_dir}");
        println!("cargo:rustc-link-lib=dylib=sqlite3");
        println!("cargo:rustc-link-lib=dylib=vec0");
        println!("cargo:rerun-if-changed=prebuilt/android/{arch}/lib");
    }

    tauri_build::build()
}

/// Minimal RFC3339 timestamp without pulling chrono into build.rs.
fn chrono_like_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Days since epoch -> civil date (Howard Hinnant's algorithm).
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}
