#[path = "src/version_manifest.rs"]
mod version_manifest;

use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_path = PathBuf::from("../version.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let raw = fs::read_to_string(&manifest_path).expect("failed to read ../version.json");
    let manifest =
        version_manifest::parse_version_manifest_str(&raw).expect("invalid version.json format");

    println!(
        "cargo:rustc-env=TASKTRACKER_BUILD_NUMBER={}",
        manifest.build_number
    );

    tauri_build::build()
}
