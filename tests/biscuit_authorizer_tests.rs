use biscuit_auth::{Biscuit, KeyPair, UnverifiedBiscuit};

#[test]
fn biscuit_authorizer_query_roundtrip() {
    // Build a keypair and a biscuit containing a user fact
    let keypair = KeyPair::new();
    let mut builder = Biscuit::builder();

    let id = "00000000-0000-0000-0000-000000000001";
    let username = "alice";
    let role = "Admin";

    let code = format!("user(\"{}\", \"{}\", \"{}\")", id, username, role);
    builder.add_code(&code).expect("add_code should succeed");

    let biscuit = builder.build(&keypair).expect("build should succeed");
    let b64 = biscuit.to_base64().expect("to_base64 should succeed");

    // Parse and verify signature
    let unverified = UnverifiedBiscuit::from_base64(&b64).expect("from_base64 should succeed");
    let biscuit = unverified
        .check_signature(|_opt_root_id| keypair.public())
        .expect("signature check should succeed");

    // Create authorizer and run
    let mut authorizer = biscuit.authorizer().expect("authorizer should be created");
    // Explicitly add an allow policy so authorize() succeeds regardless of internal defaults
    authorizer
        .add_policy("allow if true")
        .expect("add_policy should succeed");
    authorizer.authorize().expect("authorize should run");

    // Query for the user fact
    let query = r#"data($id, $name, $role) <- user($id, $name, $role)"#;
    let res: Vec<(String, String, String)> = authorizer
        .query_all(query)
        .expect("query_all should return results");

    assert_eq!(res.len(), 1, "expected exactly one user fact");
    assert_eq!(res[0].0, id);
    assert_eq!(res[0].1, username);
    assert_eq!(res[0].2, role);
}

#[test]
fn biscuit_authorizer_no_fact_returns_empty() {
    let keypair = KeyPair::new();
    let mut builder = Biscuit::builder();

    // no user fact added
    let biscuit = builder.build(&keypair).expect("build should succeed");
    let b64 = biscuit.to_base64().expect("to_base64 should succeed");

    let unverified = UnverifiedBiscuit::from_base64(&b64).expect("from_base64 should succeed");
    let biscuit = unverified
        .check_signature(|_opt_root_id| keypair.public())
        .expect("signature check should succeed");

    let mut authorizer = biscuit.authorizer().expect("authorizer should be created");
    // Make the policy explicit in this test as well
    authorizer
        .add_policy("allow if true")
        .expect("add_policy should succeed");
    authorizer.authorize().expect("authorize should run");

    let query = r#"data($id, $name, $role) <- user($id, $name, $role)"#;
    let res: Vec<(String, String, String)> = authorizer.query_all(query).expect("query_all should run");

    assert!(res.is_empty(), "expected no user facts");
}

#[test]
fn biscuit_signature_mismatch_fails() {
    let keypair = KeyPair::new();
    let other = KeyPair::new();

    let mut builder = Biscuit::builder();
    let code = r#"user("00000000-0000-0000-0000-000000000002", "bob", "Author")"#;
    builder.add_code(code).expect("add_code should succeed");

    let biscuit = builder.build(&keypair).expect("build should succeed");
    let b64 = biscuit.to_base64().expect("to_base64 should succeed");

    let unverified = UnverifiedBiscuit::from_base64(&b64).expect("from_base64 should succeed");
    // Use a different public key provider: should fail signature check
    let res = unverified.check_signature(|_opt_root_id| other.public());
    assert!(res.is_err(), "signature check should fail with wrong public key");
}
