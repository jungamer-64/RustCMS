use std::fmt::Write;

use biscuit_auth::{KeyPair, PublicKey, builder::BiscuitBuilder, error::Format as BiscuitFormat};
use chrono::Utc;
use tracing::warn;
use uuid::Uuid;

use crate::{
    Result,
    auth::error::AuthError,
    models::{User, UserRole},
    utils::common_types::SessionId,
};

pub(super) struct ParsedBiscuit {
    pub user_id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub token_type: String,
    pub session_id: SessionId,
    pub version: u32,
}

pub(super) fn build_token(
    keypair: &KeyPair,
    user: &User,
    session_id: &SessionId,
    version: u32,
    token_type: &str,
    exp_unix: i64,
) -> Result<String> {
    let mut program = String::with_capacity(256);
    writeln!(
        &mut program,
        "user(\"{}\", \"{}\", \"{}\");",
        user.id, user.username, user.role
    )
    .map_err(|e| AuthError::Biscuit(format!("Failed to generate biscuit facts: {e}")))?;
    writeln!(&mut program, "token_type(\"{token_type}\");")
        .map_err(|e| AuthError::Biscuit(format!("Failed to write token_type: {e}")))?;
    writeln!(&mut program, "exp({exp_unix});")
        .map_err(|e| AuthError::Biscuit(format!("Failed to write exp: {e}")))?;
    writeln!(
        &mut program,
        "session(\"{}\", {version});",
        session_id.as_ref()
    )
    .map_err(|e| AuthError::Biscuit(format!("Failed to write session: {e}")))?;

    let builder: BiscuitBuilder = biscuit_auth::Biscuit::builder();
    let builder = builder
        .code(&program)
        .map_err(|e| AuthError::Biscuit(format!("Failed to build biscuit facts: {e}")))?;

    let token = builder
        .build(keypair)
        .map_err(|e| AuthError::Biscuit(format!("Failed to sign biscuit: {e}")))?;

    let s = token
        .to_base64()
        .map_err(|e| crate::AppError::Biscuit(format!("Failed to serialize biscuit token: {e}")))?;
    Ok(s)
}

pub(super) fn parse_refresh_biscuit(token: &str, public_key: &PublicKey) -> Result<ParsedBiscuit> {
    let parsed = parse(token, public_key)?;
    if parsed.token_type != "refresh" {
        return Err(AuthError::InvalidToken.into());
    }
    Ok(parsed)
}

pub(super) fn parse_and_check(
    token: &str,
    expect_type: Option<&str>,
    public_key: &PublicKey,
) -> Result<ParsedBiscuit> {
    let parsed = parse(token, public_key)?;
    if let Some(t) = expect_type {
        if parsed.token_type != t {
            return Err(AuthError::InvalidToken.into());
        }
    }
    Ok(parsed)
}

fn parse(token: &str, public_key: &PublicKey) -> Result<ParsedBiscuit> {
    let mut authorizer = create_authorizer(token, public_key)?;
    let (user_id, username, role) = get_user(&mut authorizer)?;
    let token_type = get_token_type(&mut authorizer)?;
    validate_exp(&mut authorizer)?;
    let (session_id, version) = get_session(&mut authorizer)?;
    Ok(ParsedBiscuit {
        user_id,
        username,
        role,
        token_type,
        session_id,
        version,
    })
}

fn create_authorizer(token: &str, public_key: &PublicKey) -> Result<biscuit_auth::Authorizer> {
    let unverified = biscuit_auth::UnverifiedBiscuit::from_base64(token)
        .map_err(|e| AuthError::Biscuit(format!("Failed to parse biscuit token: {e}")))?;
    let key_provider = |_opt_root: Option<u32>| -> std::result::Result<PublicKey, BiscuitFormat> {
        Ok(*public_key)
    };
    let biscuit = unverified
        .verify(key_provider)
        .map_err(|e| AuthError::Biscuit(format!("Biscuit signature verification failed: {e}")))?;
    let mut authorizer = biscuit
        .authorizer()
        .map_err(|e| AuthError::Biscuit(format!("Failed to create authorizer: {e}")))?;
    authorizer
        .authorize()
        .map_err(|e| AuthError::Biscuit(format!("Authorizer run failed: {e}")))?;
    Ok(authorizer)
}

fn get_user(authorizer: &mut biscuit_auth::Authorizer) -> Result<(Uuid, String, UserRole)> {
    let (id_s, username, role_s) =
        query_triple(authorizer, "data($id,$u,$r) <- user($id,$u,$r)", "user")?;
    let user_id = Uuid::parse_str(&id_s).map_err(|_| AuthError::InvalidToken)?;
    let role = UserRole::parse_str(&role_s).map_err(|_| AuthError::InvalidToken)?;
    Ok((user_id, username, role))
}

fn get_token_type(authorizer: &mut biscuit_auth::Authorizer) -> Result<String> {
    query_string(authorizer, "data($t) <- token_type($t)", "token_type")
}

fn validate_exp(authorizer: &mut biscuit_auth::Authorizer) -> Result<()> {
    let exp = query_i64(authorizer, "data($e) <- exp($e)", "exp")?;
    let now_ts = Utc::now().timestamp();
    if exp < now_ts {
        warn!(target: "auth", "token_expired");
        return Err(AuthError::TokenExpired.into());
    }
    Ok(())
}

fn get_session(authorizer: &mut biscuit_auth::Authorizer) -> Result<(SessionId, u32)> {
    let v: Vec<(String, i64)> =
        query_vec(authorizer, "data($sid,$v) <- session($sid,$v)", "session")?;
    let (sid, ver_i) = v.into_iter().next().ok_or(AuthError::InvalidToken)?;
    let ver_u32 = u32::try_from(ver_i).map_err(|_| AuthError::InvalidToken)?;
    Ok((SessionId::from(sid), ver_u32))
}

fn query_triple(
    authz: &mut biscuit_auth::Authorizer,
    dsl: &str,
    ctx: &str,
) -> Result<(String, String, String)> {
    let v: Vec<(String, String, String)> = authz
        .query_all(dsl)
        .map_err(|e| crate::AppError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
    v.into_iter().next().ok_or(AuthError::InvalidToken.into())
}

fn query_vec(
    authz: &mut biscuit_auth::Authorizer,
    dsl: &str,
    ctx: &str,
) -> Result<Vec<(String, i64)>> {
    let v: Vec<(String, i64)> = authz
        .query_all(dsl)
        .map_err(|e| crate::AppError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
    Ok(v)
}

fn query_string(authz: &mut biscuit_auth::Authorizer, dsl: &str, ctx: &str) -> Result<String> {
    let v: Vec<(String,)> = authz
        .query_all(dsl)
        .map_err(|e| crate::AppError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
    v.into_iter()
        .next()
        .map(|t| t.0)
        .ok_or(AuthError::InvalidToken.into())
}

fn query_i64(authz: &mut biscuit_auth::Authorizer, dsl: &str, ctx: &str) -> Result<i64> {
    let v: Vec<(i64,)> = authz
        .query_all(dsl)
        .map_err(|e| crate::AppError::Biscuit(format!("Failed to query {ctx}: {e}")))?;
    v.into_iter()
        .next()
        .map(|t| t.0)
        .ok_or(AuthError::InvalidToken.into())
}
