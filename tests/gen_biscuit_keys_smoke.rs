mod helpers;

#[test]
fn smoke_prints_private_key() {
    // Use helpers to run the command and check output
    let output = std::process::Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg("Cargo.toml")
        .arg("--bin")
        .arg("gen_biscuit_keys")
        .arg("--")
        .arg("--format")
        .arg("stdout")
        .output()
        .expect("failed to run gen_biscuit_keys");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("BISCUIT_PRIVATE_KEY_B64="));
}
