#![cfg(feature = "auth")]
use biscuit_auth::{Biscuit, KeyPair};

// NOTE: biscuit-auth v6: token creation via builder pattern; authorizer().query_all() available.

fn build_token(user_id: &str, username: &str, role: &str) -> (Biscuit, KeyPair) {
    let kp = KeyPair::new();
    let fact = format!("user(\"{user_id}\", \"{username}\", \"{role}\")");
    let token = Biscuit::builder()
        .fact(fact.as_str())
        .expect("add fact")
        .build(&kp)
        .expect("build biscuit");
    (token, kp)
}

#[test]
fn biscuit_authorizer_query_roundtrip() {
    let (token, _kp) = build_token("11111111-1111-1111-1111-111111111111", "alice", "admin");
    let mut authorizer = token.authorizer().expect("authorizer");
    let q = r"data($id,$u,$r) <- user($id,$u,$r)";
    let rows: Vec<(String, String, String)> = authorizer.query_all(q).expect("query");
    assert_eq!(rows.len(), 1);
    let (id, u, r) = &rows[0];
    assert_eq!(u, "alice");
    assert_eq!(r, "admin");
    assert_eq!(id, "11111111-1111-1111-1111-111111111111");
}

#[test]
fn biscuit_authorizer_no_fact_returns_empty() {
    let kp = KeyPair::new();
    let token = Biscuit::builder().build(&kp).expect("empty biscuit");
    let mut authorizer = token.authorizer().expect("authorizer");
    let q = r"data($id,$u,$r) <- user($id,$u,$r)";
    let rows: Vec<(String, String, String)> = authorizer.query_all(q).expect("query");
    assert!(rows.is_empty(), "expected no rows, got {rows:?}");
}

#[test]
fn biscuit_signature_mismatch_fails() {
    let (token, _kp1) = build_token("00000000-0000-0000-0000-000000000000", "bob", "user");
    // Verify with different public key -> expect failure
    let kp2 = KeyPair::new();
    let serialized = token.to_base64().expect("serialize");
    let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(&serialized).expect("parse");
    let result = unverified.verify(|_| Ok(kp2.public()));
    assert!(result.is_err(), "expected signature mismatch error");
}
