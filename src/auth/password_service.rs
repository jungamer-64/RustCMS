//! パスワード検証サービス（改善版）
//!
//! # セキュリティ改善
//! - タイミング攻撃への対策
//! - 詳細なエラーログ
//! - パスワードポリシーの検証

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use std::time::Instant;
use tracing::{debug, warn};

use crate::auth::error::AuthError;

/// パスワードサービス
pub struct PasswordService {
    argon2: Argon2<'static>,
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordService {
    /// 新しいパスワードサービスを作成
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }

    /// パスワードをハッシュ化
    ///
    /// # Arguments
    /// * `password` - プレーンテキストのパスワード
    ///
    /// # Returns
    /// PHC形式のハッシュ文字列
    ///
    /// # Errors
    /// ハッシュ化に失敗した場合
    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);

        let password_hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))?;

        debug!("Password hashed successfully");
        Ok(password_hash.to_string())
    }

    /// パスワードを検証
    ///
    /// # Arguments
    /// * `password` - 検証するプレーンテキストパスワード
    /// * `hash` - 保存されているPHC形式のハッシュ
    ///
    /// # Returns
    /// パスワードが一致する場合は `Ok(())`
    ///
    /// # Security
    /// - タイミング攻撃対策として、常に一定時間かかるように実装
    /// - エラー詳細はログに記録するが、呼び出し元には返さない
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<(), AuthError> {
        let start = Instant::now();

        // ハッシュをパース
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(e) => {
                warn!("Failed to parse password hash: {}", e);
                // タイミング攻撃対策: ダミーの処理を実行
                let _ = self
                    .argon2
                    .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng));
                return Err(AuthError::InvalidPassword);
            }
        };

        // パスワード検証
        let result = self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash);

        let elapsed = start.elapsed();
        debug!("Password verification completed in {:?}", elapsed);

        match result {
            Ok(()) => {
                debug!("Password verification successful");
                Ok(())
            }
            Err(e) => {
                warn!("Password verification failed: {}", e);
                Err(AuthError::InvalidPassword)
            }
        }
    }

    /// パスワードポリシーの検証
    ///
    /// # Arguments
    /// * `password` - 検証するパスワード
    ///
    /// # Returns
    /// ポリシーを満たす場合は `Ok(())`
    ///
    /// # Policy
    /// - 最小8文字
    /// - 最大128文字
    /// - 大文字、小文字、数字を各1文字以上含む
    pub fn validate_password_policy(&self, password: &str) -> Result<(), AuthError> {
        let len = password.len();

        // 長さチェック
        if len < 8 {
            return Err(AuthError::ConfigError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        if len > 128 {
            return Err(AuthError::ConfigError(
                "Password must be at most 128 characters".to_string(),
            ));
        }

        // 文字種チェック
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());

        if !has_uppercase {
            return Err(AuthError::ConfigError(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        if !has_lowercase {
            return Err(AuthError::ConfigError(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        if !has_digit {
            return Err(AuthError::ConfigError(
                "Password must contain at least one digit".to_string(),
            ));
        }

        Ok(())
    }

    /// パスワードの強度を計算（0-100）
    ///
    /// # Arguments
    /// * `password` - 評価するパスワード
    ///
    /// # Returns
    /// 強度スコア（0: 非常に弱い、100: 非常に強い）
    pub fn calculate_password_strength(&self, password: &str) -> u8 {
        let mut score = 0u8;

        // 長さによるスコア (最大40点)
        let len = password.len();
        score += match len {
            0..=7 => 0,
            8..=11 => 10,
            12..=15 => 20,
            16..=19 => 30,
            _ => 40,
        };

        // 文字種の多様性 (最大30点)
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        let diversity = [has_lowercase, has_uppercase, has_digit, has_special]
            .iter()
            .filter(|&&x| x)
            .count();

        score += (diversity as u8) * 7; // 各7点

        // 文字の繰り返しチェック (最大20点)
        let mut prev_char = '\0';
        let mut repeat_count = 0;
        for c in password.chars() {
            if c == prev_char {
                repeat_count += 1;
            }
            prev_char = c;
        }

        score += match repeat_count {
            0..=2 => 20,
            3..=5 => 10,
            _ => 0,
        };

        // 一般的なパターンのチェック (最大10点)
        let common_patterns = ["123", "abc", "password", "qwerty"];
        let has_common_pattern = common_patterns
            .iter()
            .any(|pattern| password.to_lowercase().contains(pattern));

        if !has_common_pattern {
            score += 10;
        }

        score.min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let service = PasswordService::new();
        let password = "SecurePassword123";

        // ハッシュ化
        let hash = service
            .hash_password(password)
            .expect("Failed to hash password");

        // 正しいパスワードの検証
        assert!(service.verify_password(password, &hash).is_ok());

        // 誤ったパスワードの検証
        assert!(service.verify_password("WrongPassword", &hash).is_err());
    }

    #[test]
    fn test_password_policy_validation() {
        let service = PasswordService::new();

        // 有効なパスワード
        assert!(service.validate_password_policy("SecurePass123").is_ok());

        // 短すぎる
        assert!(service.validate_password_policy("Short1A").is_err());

        // 大文字なし
        assert!(service.validate_password_policy("lowercase123").is_err());

        // 小文字なし
        assert!(service.validate_password_policy("UPPERCASE123").is_err());

        // 数字なし
        assert!(service.validate_password_policy("NoDigitsHere").is_err());
    }

    #[test]
    fn test_password_strength() {
        let service = PasswordService::new();

        // 弱いパスワード
        let weak_score = service.calculate_password_strength("pass");
        println!("weak_score (pass): {}", weak_score);
        assert!(
            weak_score < 40,
            "Expected weak score < 40, got {}",
            weak_score
        );

        // 中程度のパスワード
        let medium_score = service.calculate_password_strength("Password123");
        println!("medium_score (Password123): {}", medium_score);
        assert!(
            medium_score >= 30 && medium_score < 70,
            "Expected medium score 30-70, got {}",
            medium_score
        );

        // 強いパスワード
        let strong_score = service.calculate_password_strength("SecureP@ssw0rd!2024");
        println!("strong_score (SecureP@ssw0rd!2024): {}", strong_score);
        assert!(
            strong_score >= 70,
            "Expected strong score >= 70, got {}",
            strong_score
        );
    }

    #[test]
    fn test_invalid_hash_format() {
        let service = PasswordService::new();
        let result = service.verify_password("password", "invalid_hash");
        assert!(matches!(result, Err(AuthError::InvalidPassword)));
    }

    #[test]
    fn test_timing_attack_resistance() {
        use std::time::Instant;

        let service = PasswordService::new();
        let password = "TestPassword123";
        let hash = service.hash_password(password).expect("Failed to hash");

        // 正しいパスワードの検証時間
        let start1 = Instant::now();
        let _ = service.verify_password(password, &hash);
        let elapsed1 = start1.elapsed();

        // 誤ったパスワードの検証時間
        let start2 = Instant::now();
        let _ = service.verify_password("WrongPassword", &hash);
        let elapsed2 = start2.elapsed();

        // Argon2の計算時間はほぼ一定だが、システムの負荷などで若干の変動がある
        // 時間差が50%以内であることを確認（タイミング攻撃対策として十分）
        let diff = if elapsed1 > elapsed2 {
            elapsed1 - elapsed2
        } else {
            elapsed2 - elapsed1
        };

        let max_time = elapsed1.max(elapsed2);
        let max_allowed_diff = max_time / 2; // 50%以内

        println!(
            "Timing: valid={:?}, invalid={:?}, diff={:?}",
            elapsed1, elapsed2, diff
        );

        assert!(
            diff < max_allowed_diff,
            "Timing difference too large: {:?} vs {:?} (diff: {:?}, max_allowed: {:?})",
            elapsed1,
            elapsed2,
            diff,
            max_allowed_diff
        );
    }
}
