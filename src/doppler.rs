/// Doppler統合モジュール
/// Dopplerから環境変数を取得し、設定を管理する

use std::collections::HashMap;
use std::process::Command;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

/// Doppler設定構造体
pub struct DopplerConfig {
    pub project: String,
    pub config: String,
    pub token: Option<String>,
}

impl Default for DopplerConfig {
    fn default() -> Self {
        Self {
            project: "cms".to_string(),
            config: "dev".to_string(),
            token: None,
        }
    }
}

/// Dopplerから環境変数を取得する（代替方法）
async fn load_from_doppler_alternative(config: &DopplerConfig) -> Result<HashMap<String, String>> {
    info!("🔐 Loading environment variables from Doppler (alternative method)...");
    
    // doppler secrets --json を使用する代替アプローチ
    let mut cmd = Command::new("doppler");
    cmd.arg("secrets")
        .arg("--json")
        .arg("--project")
        .arg(&config.project)
        .arg("--config")
        .arg(&config.config);

    if let Some(ref token) = config.token {
        cmd.arg("--token").arg(token);
    }

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed_stdout = stdout.trim();
                
                if trimmed_stdout.starts_with('{') || trimmed_stdout.starts_with('[') {
                    match serde_json::from_str::<serde_json::Value>(&trimmed_stdout) {
                        Ok(json) => {
                            let mut env_vars = HashMap::new();
                            
                            // Doppler secrets --json の出力形式に対応（メモリ効率化）
                            if let Some(obj) = json.as_object() {
                                for (key, value) in obj {
                                    // Doppler の secrets --json は { "KEY": { "computed": "value" } } 形式
                                    if let Some(secret_obj) = value.as_object() {
                                        if let Some(computed_value) = secret_obj.get("computed") {
                                            if let Some(val_str) = computed_value.as_str() {
                                                env_vars.insert(key.clone(), val_str.into()); // to_string()からinto()に変更
                                            }
                                        }
                                    } else if let Some(val_str) = value.as_str() {
                                        // シンプルな文字列値の場合
                                        env_vars.insert(key.clone(), val_str.into()); // to_string()からinto()に変更
                                    }
                                }
                            }
                            
                            info!("✅ Successfully loaded {} environment variables from Doppler (alternative)", env_vars.len());
                            Ok(env_vars)
                        },
                        Err(e) => {
                            error!("❌ Failed to parse Doppler JSON (alternative): {}", e);
                            Err(anyhow!("Failed to parse Doppler JSON (alternative): {}", e))
                        }
                    }
                } else {
                    Err(anyhow!("Doppler output is not JSON format: {}", trimmed_stdout))
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow!("Doppler command failed: {}", stderr))
            }
        },
        Err(e) => {
            Err(anyhow!("Failed to execute Doppler command: {}", e))
        }
    }
}

/// Dopplerから環境変数を取得する
pub async fn load_from_doppler(config: &DopplerConfig) -> Result<HashMap<String, String>> {
    info!("🔐 Loading environment variables from Doppler...");
    
    // まず標準の download コマンドを試す
    let mut cmd = Command::new("doppler");
    cmd.arg("secrets")
        .arg("download")
        .arg("--format")
        .arg("json")
        .arg("--no-file")  // ファイルに保存しないでstdoutに出力
        .arg("--project")
        .arg(&config.project)
        .arg("--config")
        .arg(&config.config);

    // トークンが設定されている場合は追加
    if let Some(ref token) = config.token {
        cmd.arg("--token").arg(token);
    }

    info!("📋 Using Doppler project: {}, config: {}", config.project, config.config);

    // Dopplerコマンドを実行
    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            info!("📋 Doppler command stdout length: {}", stdout.len());
            if !stderr.trim().is_empty() {
                info!("📋 Doppler command stderr: {}", stderr);
            }
            
            if output.status.success() && !stdout.trim().is_empty() {
                // JSONの開始文字をチェック
                let trimmed_stdout = stdout.trim();
                if trimmed_stdout.starts_with('{') || trimmed_stdout.starts_with('[') {
                    info!("📋 Valid JSON detected, parsing...");
                    
                    match serde_json::from_str::<serde_json::Value>(&trimmed_stdout) {
                        Ok(json) => {
                            let mut env_vars = HashMap::new();
                            
                            if let Some(obj) = json.as_object() {
                                for (key, value) in obj {
                                    if let Some(val_str) = value.as_str() {
                                        env_vars.insert(key.clone(), val_str.into()); // to_string()からinto()に変更
                                    }
                                }
                            }
                            
                            info!("✅ Successfully loaded {} environment variables from Doppler", env_vars.len());
                            Ok(env_vars)
                        },
                        Err(e) => {
                            error!("❌ Failed to parse Doppler JSON output: {}", e);
                            error!("📋 Raw stdout: {}", stdout);
                            
                            // 代替方法を試す
                            info!("🔄 Trying alternative Doppler method...");
                            match load_from_doppler_alternative(config).await {
                                Ok(vars) => Ok(vars),
                                Err(_) => {
                                    warn!("⚠️  All Doppler methods failed, falling back to .env file...");
                                    fallback_to_dotenv()
                                }
                            }
                        }
                    }
                } else {
                    warn!("⚠️  Doppler output is not JSON format: {}", trimmed_stdout);
                    
                    // 代替方法を試す
                    info!("🔄 Trying alternative Doppler method...");
                    match load_from_doppler_alternative(config).await {
                        Ok(vars) => Ok(vars),
                        Err(_) => {
                            warn!("⚠️  All Doppler methods failed, falling back to .env file...");
                            fallback_to_dotenv()
                        }
                    }
                }
            } else {
                warn!("⚠️  Doppler command failed or returned empty output");
                warn!("📋 Status: {}, stdout empty: {}", output.status, stdout.trim().is_empty());
                if !stderr.trim().is_empty() {
                    warn!("📋 stderr: {}", stderr);
                }
                
                // 代替方法を試す
                info!("🔄 Trying alternative Doppler method...");
                match load_from_doppler_alternative(config).await {
                    Ok(vars) => Ok(vars),
                    Err(_) => {
                        warn!("⚠️  All Doppler methods failed, falling back to .env file...");
                        fallback_to_dotenv()
                    }
                }
            }
        },
        Err(e) => {
            error!("❌ Failed to execute Doppler command: {}", e);
            warn!("⚠️  Falling back to .env file...");
            fallback_to_dotenv()
        }
    }
}

/// Dopplerが利用できない場合の.envファイルフォールバック
fn fallback_to_dotenv() -> Result<HashMap<String, String>> {
    info!("📁 Loading from .env file as fallback...");
    
    dotenv::dotenv().ok();
    
    let mut env_vars = HashMap::new();
    
    // 必要な環境変数をチェック
    let required_vars = [
        "HOST", "PORT", "DATABASE_URL", "DATABASE_NAME",
        "JWT_SECRET", "JWT_EXPIRATION_HOURS", "JWT_REFRESH_EXPIRATION_DAYS",
        "ALLOWED_ORIGINS", "UPLOAD_DIR"
    ];
    
    for var in &required_vars {
        if let Ok(value) = std::env::var(var) {
            env_vars.insert(var.to_string(), value);
        }
    }
    
    if env_vars.is_empty() {
        return Err(anyhow!("No environment variables found in .env file"));
    }
    
    info!("✅ Loaded {} variables from .env file", env_vars.len());
    Ok(env_vars)
}

/// 環境変数をシステム環境に設定
pub fn set_environment_variables(env_vars: HashMap<String, String>) {
    info!("🔧 Setting environment variables...");
    
    for (key, value) in env_vars {
        std::env::set_var(&key, &value);
        // セキュリティのため、機密情報は表示しない
        if key.to_lowercase().contains("secret") || key.to_lowercase().contains("token") {
            info!("  {} = [HIDDEN]", key);
        } else {
            info!("  {} = {}", key, value);
        }
    }
}

/// Dopplerの設定をテストする
pub async fn test_doppler_connection(config: &DopplerConfig) -> bool {
    info!("🧪 Testing Doppler connection...");
    
    // まずDoppler CLIの存在確認
    let mut cmd = Command::new("doppler");
    cmd.arg("--version");
    
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("✅ Doppler CLI version: {}", version.trim());
                
                // 次に認証状態をチェック
                let mut auth_cmd = Command::new("doppler");
                auth_cmd.arg("me");
                
                match auth_cmd.output() {
                    Ok(auth_output) => {
                        if auth_output.status.success() {
                            info!("✅ Doppler authentication verified");
                            
                            // プロジェクト設定をテスト
                            let mut test_cmd = Command::new("doppler");
                            test_cmd.arg("secrets")
                                .arg("--project")
                                .arg(&config.project)
                                .arg("--config")
                                .arg(&config.config)
                                .arg("--only-names");
                            
                            match test_cmd.output() {
                                Ok(test_output) => {
                                    if test_output.status.success() {
                                        info!("✅ Doppler project access confirmed");
                                        true
                                    } else {
                                        let stderr = String::from_utf8_lossy(&test_output.stderr);
                                        warn!("⚠️  Cannot access Doppler project: {}", stderr);
                                        false
                                    }
                                },
                                Err(e) => {
                                    warn!("⚠️  Cannot test Doppler project access: {}", e);
                                    false
                                }
                            }
                        } else {
                            let stderr = String::from_utf8_lossy(&auth_output.stderr);
                            warn!("⚠️  Doppler authentication failed: {}", stderr);
                            false
                        }
                    },
                    Err(e) => {
                        warn!("⚠️  Cannot check Doppler authentication: {}", e);
                        false
                    }
                }
            } else {
                warn!("⚠️  Doppler CLI not responding properly");
                false
            }
        },
        Err(_) => {
            warn!("⚠️  Doppler CLI not found in PATH");
            false
        }
    }
}
