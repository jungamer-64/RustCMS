// Integration test: invalid/expired biscuit token handling

#[cfg(test)]
mod tests {
    use biscuit_auth::{KeyPair, PublicKey, UnverifiedBiscuit};

    #[test]
    fn invalid_biscuit_token_rejected() {
        // Create a biscuit signed with one keypair
        let kp = KeyPair::new();
        let priv_key = kp.private();

        let program = format!(
            "user(\"{}\", \"{}\", \"{}\"); token_type(\"{}\"); exp({}); session(\"{}\", {});",
            "00000000-0000-0000-0000-000000000000",
            "u",
            "Subscriber",
            "access",
            9_999_999_999i64,
            "s",
            1
        );

        let builder = biscuit_auth::Biscuit::builder();
        let builder = builder.code(&program).expect("build facts");
        let keypair = KeyPair::from(&priv_key);
        let token = builder.build(&keypair).expect("sign biscuit");
        let b64 = token.to_base64().expect("serialize biscuit");

        // Attempt to verify using a different public key -> should fail
        let kp2 = KeyPair::new();
        let pub2 = kp2.public();

        let key_provider = |_opt_root_id: Option<u32>| -> std::result::Result<PublicKey, biscuit_auth::error::Format> { Ok(pub2) };

        let unverified = UnverifiedBiscuit::from_base64(&b64).expect("parse base64");
        let res = unverified.verify(key_provider);
        assert!(
            res.is_err(),
            "Expected verification to fail with wrong public key"
        );
    }
}
