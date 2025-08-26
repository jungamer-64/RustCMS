use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn smoke_prints_private_key() {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--package").arg("cms-backend").arg("--bin").arg("gen_biscuit_keys").arg("--")
        .arg("--format").arg("stdout");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("BISCUIT_PRIVATE_KEY_B64="));
}
