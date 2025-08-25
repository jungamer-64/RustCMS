use crate::utils::error::{AppError, AppResult};
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use urlencoding::{decode, encode};

/// URL-safe文字セット定義（RFC 3986準拠）
/// CONTROLS（制御文字）+ 特定の文字を追加
const URL_SAFE: &AsciiSet = &CONTROLS
    .add(b' ') // スペース
    .add(b'"') // ダブルクォート
    .add(b'<') // 小なり
    .add(b'>') // 大なり
    .add(b'`') // バッククォート
    .add(b'#') // ハッシュ
    .add(b'?') // クエスチョン
    .add(b'{') // 左中括弧
    .add(b'}') // 右中括弧
    .add(b'%') // パーセント（二重エンコード防止）
    .add(b'/') // スラッシュ
    .add(b'\\'); // バックスラッシュ

/// スラッグ用文字セット（より厳密）
const SLUG_SAFE: &AsciiSet = &URL_SAFE
    .add(b'!') // 感嘆符
    .add(b'*') // アスタリスク
    .add(b'\'') // シングルクォート
    .add(b'(') // 左括弧
    .add(b')') // 右括弧
    .add(b';') // セミコロン
    .add(b':') // コロン
    .add(b'@') // アットマーク
    .add(b'&') // アンパサンド
    .add(b'=') // イコール
    .add(b'+') // プラス
    .add(b'$') // ドル
    .add(b',') // カンマ
    .add(b'[') // 左角括弧
    .add(b']'); // 右角括弧

/// URLパラメータの安全なエンコード
pub fn encode_url_param(input: &str) -> String {
    if is_already_encoded(input) {
        input.to_string()
    } else {
        utf8_percent_encode(input, URL_SAFE).to_string()
    }
}

/// URLパラメータの安全なデコード
pub fn decode_url_param(input: &str) -> AppResult<String> {
    percent_decode_str(input)
        .decode_utf8()
        .map_err(|e| AppError::BadRequest(format!("Invalid URL encoding: {}", e)))
        .map(|cow| cow.into_owned())
}

/// スラッグの安全なエンコード（より厳密）
pub fn encode_slug(input: &str) -> String {
    if is_already_encoded(input) {
        input.to_string()
    } else {
        utf8_percent_encode(input, SLUG_SAFE).to_string()
    }
}

/// スラッグの安全なデコード
pub fn decode_slug(input: &str) -> AppResult<String> {
    percent_decode_str(input)
        .decode_utf8()
        .map_err(|e| AppError::BadRequest(format!("Invalid slug encoding: {}", e)))
        .map(|cow| cow.into_owned())
}

/// 簡易URLエンコード（完全互換）
pub fn url_encode(input: &str) -> String {
    encode(input).to_string()
}

/// 簡易URLデコード（完全互換）
pub fn url_decode(input: &str) -> AppResult<String> {
    decode(input)
        .map_err(|e| AppError::BadRequest(format!("URL decode error: {}", e)))
        .map(|cow| cow.into_owned())
}

/// 文字列が既にURLエンコードされているかチェック
fn is_already_encoded(input: &str) -> bool {
    // %xx形式のパターンが含まれているかチェック
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            // 次の2文字が16進数かチェック
            if let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
                if c1.is_ascii_hexdigit() && c2.is_ascii_hexdigit() {
                    return true;
                }
            }
        }
    }
    false
}

/// パラメーター長さのバリデーション
pub fn validate_param_length(param: &str, max_length: usize, param_name: &str) -> AppResult<()> {
    if param.len() > max_length {
        return Err(AppError::BadRequest(format!(
            "{} is too long: {} characters (max: {})",
            param_name,
            param.len(),
            max_length
        )));
    }
    Ok(())
}

/// 安全なスラッグ生成（日本語対応）
pub fn generate_safe_slug(title: &str) -> String {
    // 1. 基本的な文字列正規化
    let normalized = title
        .trim()
        .to_lowercase()
        .replace([' ', '\t', '\n', '\r'], "-") // 空白文字をハイフンに
        .replace(['/', '\\', '?', '#', '&'], "-") // 危険な文字をハイフンに
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || !c.is_ascii()) // 日本語文字は保持
        .collect::<String>();

    // 2. 連続するハイフンを単一に
    let cleaned = normalized
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // 3. URLエンコード（非ASCII文字のみ）
    if !cleaned.is_ascii() {
        encode_slug(&cleaned)
    } else {
        cleaned
    }
}

/// URL安全性の検証
pub fn validate_url_param(input: &str) -> AppResult<()> {
    // 最大長チェック
    if input.len() > 2048 {
        return Err(AppError::BadRequest("URL parameter too long".to_string()));
    }

    // 無効な文字パターンチェック
    if input.contains('\0') || input.contains('\u{FFFF}') {
        return Err(AppError::BadRequest(
            "Invalid characters in URL parameter".to_string(),
        ));
    }

    // デコードテスト
    decode_url_param(input)?;

    Ok(())
}

/// スラッグ安全性の検証
pub fn validate_slug(input: &str) -> AppResult<()> {
    // 最大長チェック
    if input.len() > 255 {
        return Err(AppError::BadRequest("Slug too long".to_string()));
    }

    // 空文字チェック
    if input.trim().is_empty() {
        return Err(AppError::BadRequest("Slug cannot be empty".to_string()));
    }

    // 無効な文字パターンチェック
    if input.contains('\0') || input.contains('\u{FFFF}') {
        return Err(AppError::BadRequest(
            "Invalid characters in slug".to_string(),
        ));
    }

    // デコードテスト
    decode_slug(input)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_encoding() {
        assert_eq!(encode_url_param("hello world"), "hello%20world");
        assert_eq!(decode_url_param("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn test_japanese_encoding() {
        let japanese = "こんにちは世界";
        let encoded = encode_url_param(japanese);
        let decoded = decode_url_param(&encoded).unwrap();
        assert_eq!(decoded, japanese);
    }

    #[test]
    fn test_slug_generation() {
        assert_eq!(generate_safe_slug("Hello World!"), "hello-world");
        assert_eq!(
            generate_safe_slug("こんにちは 世界"),
            encode_slug("こんにちは-世界")
        );
    }

    #[test]
    fn test_double_encoding_prevention() {
        let already_encoded = "hello%20world";
        assert_eq!(encode_url_param(already_encoded), already_encoded);
    }

    #[test]
    fn test_validation() {
        assert!(validate_url_param("valid-param").is_ok());
        assert!(validate_slug("valid-slug").is_ok());
        assert!(validate_url_param(&"x".repeat(3000)).is_err());
        assert!(validate_slug(&"x".repeat(300)).is_err());
    }
}
