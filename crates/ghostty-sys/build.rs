use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const FRAMEWORKS: &[&str] = &[
    "AppKit",
    "Carbon",
    "CoreFoundation",
    "CoreGraphics",
    "CoreServices",
    "CoreText",
    "CoreVideo",
    "Foundation",
    "IOSurface",
    "Metal",
    "MetalKit",
    "QuartzCore",
    "UniformTypeIdentifiers",
];

fn main() {
    let target = env::var("TARGET").expect("TARGET");
    assert_eq!(
        target, "aarch64-apple-darwin",
        "combe builds only for aarch64-apple-darwin, got {target}"
    );

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ghostty = manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("vendor/ghostty");
    assert!(
        ghostty.join("build.zig").is_file(),
        "vendor/ghostty is empty; run: git submodule update --init"
    );

    println!(
        "cargo:rerun-if-changed={}",
        ghostty.join("build.zig").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ghostty.join("build.zig.zon").display()
    );
    println!("cargo:rerun-if-env-changed=ZIG");

    let zig = env::var("ZIG").unwrap_or_else(|_| "zig".into());
    require_zig(&zig, &required_zig_version(&ghostty));

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let prefix = out.join("ghostty");
    let cache = ghostty
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join(".local/zig-cache");
    build_ghostty(&zig, &ghostty, &prefix, &cache);

    let lib_dir = ghostty.join("macos/GhosttyKit.xcframework/macos-arm64");
    assert!(
        lib_dir.join("libghostty-internal.a").is_file(),
        "expected libghostty-internal.a in {}",
        lib_dir.display()
    );
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=ghostty-internal");
    println!("cargo:rustc-link-lib=c++");
    for framework in FRAMEWORKS {
        println!("cargo:rustc-link-lib=framework={framework}");
    }

    let resources = prefix.join("share/ghostty");
    println!(
        "cargo:rustc-env=GHOSTTY_RESOURCES_DIR={}",
        resources.display()
    );

    let header = ghostty.join("include/ghostty.h");
    println!("cargo:rerun-if-changed={}", header.display());
    bindgen::Builder::default()
        .header(header.to_str().expect("header path"))
        .allowlist_item("ghostty_.*")
        .allowlist_item("GHOSTTY_.*")
        .prepend_enum_name(false)
        .derive_default(true)
        .layout_tests(false)
        .generate()
        .expect("bindgen")
        .write_to_file(out.join("bindings.rs"))
        .expect("write bindings");
}

fn required_zig_version(ghostty: &Path) -> String {
    let manifest =
        std::fs::read_to_string(ghostty.join("build.zig.zon")).expect("read build.zig.zon");
    manifest
        .lines()
        .find_map(|line| {
            let rest = line.trim().strip_prefix(".minimum_zig_version")?;
            let start = rest.find('"')? + 1;
            let end = rest[start..].find('"')? + start;
            Some(rest[start..end].to_string())
        })
        .expect("minimum_zig_version in build.zig.zon")
}

fn require_zig(zig: &str, want: &str) {
    let output = Command::new(zig)
        .arg("version")
        .output()
        .unwrap_or_else(|err| {
            panic!("cannot run `{zig} version`: {err}. Install zig {want} (brew install zig)")
        });
    let have = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(
        have, want,
        "vendor/ghostty needs zig {want}, found {have}. Set ZIG=/path/to/zig or install zig {want}"
    );
}

fn build_ghostty(zig: &str, ghostty: &Path, prefix: &Path, cache: &Path) {
    let optimize = match env::var("PROFILE").as_deref() {
        Ok("release") => "ReleaseFast",
        _ => "Debug",
    };
    let status = Command::new(zig)
        .current_dir(ghostty)
        .arg("build")
        .arg("-Dapp-runtime=none")
        .arg("-Dxcframework-target=native")
        .arg("-Demit-xcframework=true")
        .arg("-Demit-macos-app=false")
        .arg(format!("-Doptimize={optimize}"))
        .arg("--prefix")
        .arg(prefix)
        .arg("--cache-dir")
        .arg(cache)
        .status()
        .expect("run zig build");
    assert!(status.success(), "zig build failed: {status}");
}
