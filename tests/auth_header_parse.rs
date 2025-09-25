use cms_backend::middleware::auth::parse_authorization_header;

mod common;

#[test]
fn bearer_ok() {
    common::setup();
    let t = parse_authorization_header("Bearer abc").expect("should parse bearer token");
    assert_eq!(t, "abc");
}

#[test]
fn biscuit_ok() {
    common::setup();
    let t = parse_authorization_header("Biscuit xyz").expect("should parse biscuit token");
    assert_eq!(t, "xyz");
}

#[test]
fn invalid_scheme() {
    common::setup();
    assert!(
        parse_authorization_header("Basic a:b").is_none(),
        "Basic scheme should be rejected"
    );
}

#[test]
fn extra_spaces_trimmed() {
    common::setup();
    let t = parse_authorization_header("  Bearer   tok  ").expect("should trim spaces");
    assert_eq!(t, "tok");
}
