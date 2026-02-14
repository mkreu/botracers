use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

pub const BOT_TARGET_TRIPLE: &str = "riscv32imafc-unknown-none-elf";

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
struct CargoPackage {
    targets: Vec<CargoTarget>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

pub fn bot_project_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../bot")
}

fn bot_manifest_path() -> PathBuf {
    bot_project_dir().join("Cargo.toml")
}

pub fn discover_bot_binaries() -> Result<Vec<String>, String> {
    let manifest = bot_manifest_path();
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .output()
        .map_err(|error| format!("failed to run cargo metadata: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {}", stderr.trim()));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid cargo metadata JSON: {error}"))?;

    let mut binaries: Vec<String> = metadata
        .packages
        .into_iter()
        .flat_map(|package| package.targets)
        .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
        .map(|target| target.name)
        .collect();

    binaries.sort();
    binaries.dedup();

    Ok(binaries)
}

pub fn compile_bot_binary_and_read_elf(bot_dir: &Path, binary: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin")
        .arg(binary)
        .arg("--target")
        .arg(BOT_TARGET_TRIPLE)
        .current_dir(bot_dir)
        .output()
        .map_err(|error| format!("failed to run cargo build for '{binary}': {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(details);
    }

    let elf_path = bot_dir
        .join("target")
        .join(BOT_TARGET_TRIPLE)
        .join("release")
        .join(binary);

    std::fs::read(&elf_path)
        .map_err(|error| format!("failed to read ELF '{}': {error}", elf_path.display()))
}
