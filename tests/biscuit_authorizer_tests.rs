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
