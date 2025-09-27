use assert_cmd::Command;

#[test]
fn cms_admin_help_contains_expected_sections() {
    // Invoke the binary help through cargo run to ensure the same binary is used.
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-q", "--bin", "cms-admin", "--", "--help"]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let s = String::from_utf8_lossy(&output).to_string();

    // Basic sanity checks on help output to ensure CLI layout is stable.
    assert!(s.contains("Usage"), "help should contain Usage");
    assert!(s.contains("Commands:"), "help should list Commands");
    assert!(s.contains("user"), "help should mention user subcommand");
    assert!(s.contains("content"), "help should mention content subcommand");
    assert!(s.contains("--help"), "help should show help flag");
}
