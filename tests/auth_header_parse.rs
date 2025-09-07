use cms_backend::middleware::auth::parse_authorization_header;

#[test]
fn bearer_ok() {
    let t = parse_authorization_header("Bearer abc").unwrap();
    assert_eq!(t, "abc");
}

#[test]
fn biscuit_ok() {
    let t = parse_authorization_header("Biscuit xyz").unwrap();
    assert_eq!(t, "xyz");
}

#[test]
fn invalid_scheme() {
    assert!(parse_authorization_header("Basic a:b").is_none());
}
