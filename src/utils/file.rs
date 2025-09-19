use chrono::Datelike;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

#[derive(Debug)]
pub enum FileError {
    IoError(io::Error),
    InvalidFileType,
    FileTooLarge,
    InvalidPath,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::InvalidFileType => write!(f, "Invalid file type"),
            Self::FileTooLarge => write!(f, "File too large"),
            Self::InvalidPath => write!(f, "Invalid file path"),
        }
    }
}

impl std::error::Error for FileError {}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

/// 許可されたファイル拡張子
pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];
pub const ALLOWED_DOCUMENT_EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "txt", "md"];
pub const ALLOWED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "ogg"];

/// 最大ファイルサイズ (バイト)
pub const MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
pub const MAX_DOCUMENT_SIZE: u64 = 50 * 1024 * 1024; // 50MB
pub const MAX_VIDEO_SIZE: u64 = 500 * 1024 * 1024; // 500MB

/// ファイル情報
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub mime_type: String,
    pub hash: String,
}

/// ファイルの拡張子を取得
pub fn get_file_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
}

/// ファイルタイプを検証
///
/// # Errors
///
/// `filename` の拡張子が `allowed_extensions` に含まれない場合、`InvalidFileType` を返します。
pub fn validate_file_type(filename: &str, allowed_extensions: &[&str]) -> Result<(), FileError> {
    match get_file_extension(filename) {
        Some(ext) if allowed_extensions.contains(&ext.as_str()) => Ok(()),
        _ => Err(FileError::InvalidFileType),
    }
}

/// ファイルサイズを検証
///
/// # Errors
///
/// `size` が `max_size` を超える場合、`FileTooLarge` を返します。
pub const fn validate_file_size(size: u64, max_size: u64) -> Result<(), FileError> {
    if size > max_size {
        return Err(FileError::FileTooLarge);
    }
    Ok(())
}

/// ファイルのハッシュを計算
///
/// # Errors
///
/// ファイルの読み取りに失敗した場合、`IoError` を返します。
pub fn calculate_file_hash(file_path: &Path) -> Result<String, FileError> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// 非同期でファイルのハッシュを計算
///
/// # Errors
///
/// ファイルの読み取りに失敗した場合、`IoError` を返します。
pub async fn calculate_file_hash_async(file_path: &Path) -> Result<String, FileError> {
    let mut file = async_fs::File::open(file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// 安全なファイル名を生成
#[must_use]
pub fn generate_safe_filename(original_name: &str) -> String {
    let uuid = Uuid::new_v4();
    let extension = get_file_extension(original_name).unwrap_or_default();

    if extension.is_empty() {
        uuid.to_string()
    } else {
        format!("{uuid}.{extension}")
    }
}

/// ディレクトリパスを作成
///
/// # Errors
///
/// ディレクトリ作成に失敗した場合、`IoError` を返します。
pub fn ensure_directory_exists(path: &Path) -> Result<(), FileError> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// 非同期でディレクトリパスを作成
///
/// # Errors
///
/// ディレクトリ作成に失敗した場合、`IoError` を返します。
pub async fn ensure_directory_exists_async(path: &Path) -> Result<(), FileError> {
    if !path.exists() {
        async_fs::create_dir_all(path).await?;
    }
    Ok(())
}

/// ファイルを移動
///
/// # Errors
///
/// 移動元/先の存在や権限不足などで失敗した場合、`IoError` を返します。
pub fn move_file(from: &Path, to: &Path) -> Result<(), FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

/// 非同期でファイルを移動
///
/// # Errors
///
/// 移動元/先の存在や権限不足などで失敗した場合、`IoError` を返します。
pub async fn move_file_async(from: &Path, to: &Path) -> Result<(), FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists_async(parent).await?;
    }
    async_fs::rename(from, to).await?;
    Ok(())
}

/// ファイルをコピー
///
/// # Errors
///
/// コピー時の入出力に失敗した場合、`IoError` を返します。
pub fn copy_file(from: &Path, to: &Path) -> Result<u64, FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists(parent)?;
    }
    let bytes_copied = fs::copy(from, to)?;
    Ok(bytes_copied)
}

/// 非同期でファイルをコピー
///
/// # Errors
///
/// コピー時の入出力に失敗した場合、`IoError` を返します。
pub async fn copy_file_async(from: &Path, to: &Path) -> Result<u64, FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists_async(parent).await?;
    }
    let bytes_copied = async_fs::copy(from, to).await?;
    Ok(bytes_copied)
}

/// ファイルを削除
///
/// # Errors
///
/// 削除対象が存在しない、または権限不足などで失敗した場合、`IoError` を返します。
pub fn delete_file(path: &Path) -> Result<(), FileError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// 非同期でファイルを削除
///
/// # Errors
///
/// 削除対象が存在しない、または権限不足などで失敗した場合、`IoError` を返します。
pub async fn delete_file_async(path: &Path) -> Result<(), FileError> {
    if path.exists() {
        async_fs::remove_file(path).await?;
    }
    Ok(())
}

/// ファイル情報を取得
///
/// # Errors
///
/// メタデータ取得やハッシュ計算に失敗した場合、`IoError` を返します。
pub fn get_file_info(path: &Path) -> Result<FileInfo, FileError> {
    let metadata = fs::metadata(path)?;
    build_file_info_from_parts(path, metadata.len(), || calculate_file_hash(path))
}

/// 非同期でファイル情報を取得
///
/// # Errors
///
/// メタデータの取得やハッシュ計算に失敗した場合、`FileError` を返します。
pub async fn get_file_info_async(path: &Path) -> Result<FileInfo, FileError> {
    let metadata = async_fs::metadata(path).await?;
    // Adapt sync builder by deferring only the hash as async
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FileError::InvalidPath)?;
    let extension = get_file_extension(filename).unwrap_or_default();
    let mime_type = get_mime_type(&extension);
    let hash = calculate_file_hash_async(path).await?;
    Ok(FileInfo {
        name: filename.to_string(),
        extension,
        size: metadata.len(),
        mime_type,
        hash,
    })
}

/// Internal helper: assemble `FileInfo` given a path, size and a hash supplier.
fn build_file_info_from_parts<F>(path: &Path, size: u64, hash_fn: F) -> Result<FileInfo, FileError>
where
    F: FnOnce() -> Result<String, FileError>,
{
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FileError::InvalidPath)?;
    let extension = get_file_extension(filename).unwrap_or_default();
    let mime_type = get_mime_type(&extension);
    let hash = hash_fn()?;
    Ok(FileInfo {
        name: filename.to_string(),
        extension,
        size,
        mime_type,
        hash,
    })
}

/// 拡張子からMIMEタイプを取得
#[must_use]
pub fn get_mime_type(extension: &str) -> String {
    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "video/ogg",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// ファイルサイズを人間が読みやすい形式に変換
#[must_use]
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    // Avoid precision loss by operating on integers and formatting at the end
    let mut value = bytes;
    let mut unit_index = 0;

    while value >= 1024 && unit_index < UNITS.len() - 1 {
        value /= 1024;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        // Compute two decimal places using integer arithmetic to avoid precision loss
        let pow = 1024u64.pow(u32::try_from(unit_index).unwrap_or(0));
        let whole = bytes / pow;
        let rem = bytes % pow;
        // Scale remainder to hundredths
        let hundredths = (rem * 100 + (pow / 20)) / pow; // round to nearest
        format!("{whole}.{hundredths:02} {}", UNITS[unit_index])
    }
}

/// アップロード用のパスを生成
#[must_use]
pub fn generate_upload_path(base_dir: &Path, category: &str, filename: &str) -> PathBuf {
    let safe_filename = generate_safe_filename(filename);
    let date = chrono::Utc::now();
    let year = date.year();
    let month = date.month();

    base_dir
        .join(category)
        .join(year.to_string())
        .join(format!("{month:02}"))
        .join(safe_filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_file_extension() {
        assert_eq!(get_file_extension("test.jpg"), Some("jpg".to_string()));
        assert_eq!(get_file_extension("test.PDF"), Some("pdf".to_string()));
        assert_eq!(get_file_extension("test"), None);
    }

    #[test]
    fn test_validate_file_type() {
        assert!(validate_file_type("test.jpg", ALLOWED_IMAGE_EXTENSIONS).is_ok());
        assert!(validate_file_type("test.txt", ALLOWED_IMAGE_EXTENSIONS).is_err());
    }

    #[test]
    fn test_validate_file_size() {
        assert!(validate_file_size(1000, MAX_IMAGE_SIZE).is_ok());
        assert!(validate_file_size(MAX_IMAGE_SIZE + 1, MAX_IMAGE_SIZE).is_err());
    }

    #[test]
    fn test_generate_safe_filename() {
        let safe_name = generate_safe_filename("test file.jpg");
        assert!(
            std::path::Path::new(&safe_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg"))
        );
        assert!(!safe_name.contains(' '));
    }

    #[test]
    fn test_get_mime_type() {
        assert_eq!(get_mime_type("jpg"), "image/jpeg");
        assert_eq!(get_mime_type("pdf"), "application/pdf");
        assert_eq!(get_mime_type("unknown"), "application/octet-stream");
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1_048_576), "1.00 MB");
        assert_eq!(format_file_size(500), "500 B");
    }

    #[test]
    fn test_calculate_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test content").unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();
        assert_eq!(hash.len(), 64); // SHA256は64文字
    }
}
