use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use tempfile::tempdir;

// Number of versioned generations to perform before checking prune retention.
// Reduced to speed up CI while still exercising prune & manifest logic
const GENERATIONS: usize = 3;
const PRUNE_KEEP: usize = 2;

#[test]
fn versioned_keys_manifest_and_prune() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("keys");
    fs::create_dir_all(&out_dir).unwrap();

    // Run multiple generations with --versioned and prune retention.
    for _ in 0..GENERATIONS {
        let mut cmd = Command::cargo_bin("gen_biscuit_keys").expect("binary build");
        cmd.arg("--format")
            .arg("files")
            .arg("--out-dir")
            .arg(out_dir.to_string_lossy().as_ref())
            .arg("--versioned")
            .arg("--latest-alias")
            .arg("--prune")
            .arg(PRUNE_KEEP.to_string())
            .arg("--force");
        cmd.assert().success();
    }

    // Collect versioned private key files.
    let mut versions: Vec<u32> = Vec::new();
    for entry in fs::read_dir(&out_dir).expect("should read test output dir") {
        let entry = entry.expect("should read dir entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(v) = parse_version(&name) {
            versions.push(v);
        }
    }
    let distinct: HashSet<u32> = versions.iter().copied().collect();
    assert!(
        distinct.len() <= PRUNE_KEEP,
        "expected at most {PRUNE_KEEP} distinct versions, found {}",
        distinct.len()
    );
    assert!(!distinct.is_empty(), "expected some versioned files");
    let mut distinct_vec: Vec<u32> = distinct.into_iter().collect();
    distinct_vec.sort_unstable();
    let max_version = *distinct_vec.last().expect("should have at least one version");

    // Validate latest alias files exist.
    assert!(
        out_dir.join("biscuit_private.b64").exists(),
        "missing unversioned private alias"
    );
    assert!(
        out_dir.join("biscuit_public.b64").exists(),
        "missing unversioned public alias"
    );

    // Read manifest.json and validate JSON fields.
    let manifest_path = out_dir.join("manifest.json");
    assert!(manifest_path.exists(), "manifest.json not written");
    let mut mf = fs::File::open(&manifest_path).expect("should open manifest");
    let mut buf = String::new();
    mf.read_to_string(&mut buf).expect("should read manifest to string");
    let v: Value = serde_json::from_str(&buf).expect("manifest.json invalid JSON");
    let latest_version_u64 = v["latest_version"]
        .as_u64()
        .expect("latest_version missing / not number");
    let latest_version =
        u32::try_from(latest_version_u64).expect("latest_version value too large for u32");
    assert_eq!(
        latest_version, max_version,
        "manifest latest_version mismatch"
    );
    assert!(
        v["private_fingerprint"].as_str().unwrap_or("").len() >= 32,
        "private_fingerprint too short"
    );
    assert!(
        v["public_fingerprint"].as_str().unwrap_or("").len() >= 32,
        "public_fingerprint too short"
    );

    // Run --list and ensure it outputs versions.
    let mut list_cmd = Command::cargo_bin("gen_biscuit_keys").expect("binary build");
    list_cmd
        .arg("--out-dir")
        .arg(out_dir.to_string_lossy().as_ref())
        .arg("--list");
    list_cmd
        .assert()
        .success()
        .stdout(contains("Found versions"));
}

// Re-implement parse_version from the binary for test-side inspection.
fn parse_version(name: &str) -> Option<u32> {
    if let Some(idx) = name.rfind("_v") {
        let ver_part = &name[idx + 2..];
        if let Some(dot) = ver_part.find('.') {
            return ver_part[..dot].parse().ok();
        }
    }
    None
}
