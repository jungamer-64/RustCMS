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
            FileError::IoError(e) => write!(f, "IO error: {}", e),
            FileError::InvalidFileType => write!(f, "Invalid file type"),
            FileError::FileTooLarge => write!(f, "File too large"),
            FileError::InvalidPath => write!(f, "Invalid file path"),
        }
    }
}

impl std::error::Error for FileError {}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> Self {
        FileError::IoError(err)
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
        .map(|ext| ext.to_lowercase())
}

/// ファイルタイプを検証
pub fn validate_file_type(filename: &str, allowed_extensions: &[&str]) -> Result<(), FileError> {
    match get_file_extension(filename) {
        Some(ext) if allowed_extensions.contains(&ext.as_str()) => Ok(()),
        _ => Err(FileError::InvalidFileType),
    }
}

/// ファイルサイズを検証
pub fn validate_file_size(size: u64, max_size: u64) -> Result<(), FileError> {
    if size > max_size {
        Err(FileError::FileTooLarge)
    } else {
        Ok(())
    }
}

/// ファイルのハッシュを計算
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

    Ok(format!("{value:x}", value = hasher.finalize()))
}

/// 非同期でファイルのハッシュを計算
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

    Ok(format!("{value:x}", value = hasher.finalize()))
}

/// 安全なファイル名を生成
pub fn generate_safe_filename(original_name: &str) -> String {
    let uuid = Uuid::new_v4();
    let extension = get_file_extension(original_name).unwrap_or_default();

    if extension.is_empty() {
        uuid.to_string()
    } else {
    format!("{uuid}.{extension}", uuid = uuid, extension = extension)
    }
}

/// ディレクトリパスを作成
pub fn ensure_directory_exists(path: &Path) -> Result<(), FileError> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// 非同期でディレクトリパスを作成
pub async fn ensure_directory_exists_async(path: &Path) -> Result<(), FileError> {
    if !path.exists() {
        async_fs::create_dir_all(path).await?;
    }
    Ok(())
}

/// ファイルを移動
pub fn move_file(from: &Path, to: &Path) -> Result<(), FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

/// 非同期でファイルを移動
pub async fn move_file_async(from: &Path, to: &Path) -> Result<(), FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists_async(parent).await?;
    }
    async_fs::rename(from, to).await?;
    Ok(())
}

/// ファイルをコピー
pub fn copy_file(from: &Path, to: &Path) -> Result<u64, FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists(parent)?;
    }
    let bytes_copied = fs::copy(from, to)?;
    Ok(bytes_copied)
}

/// 非同期でファイルをコピー
pub async fn copy_file_async(from: &Path, to: &Path) -> Result<u64, FileError> {
    if let Some(parent) = to.parent() {
        ensure_directory_exists_async(parent).await?;
    }
    let bytes_copied = async_fs::copy(from, to).await?;
    Ok(bytes_copied)
}

/// ファイルを削除
pub fn delete_file(path: &Path) -> Result<(), FileError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// 非同期でファイルを削除
pub async fn delete_file_async(path: &Path) -> Result<(), FileError> {
    if path.exists() {
        async_fs::remove_file(path).await?;
    }
    Ok(())
}

/// ファイル情報を取得
pub fn get_file_info(path: &Path) -> Result<FileInfo, FileError> {
    let metadata = fs::metadata(path)?;
    build_file_info_from_parts(path, metadata.len(), || {
        calculate_file_hash(path)
    })
}

/// 非同期でファイル情報を取得
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

/// Internal helper: assemble FileInfo given a path, size and a hash supplier.
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
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
    format!("{bytes} {unit}", bytes = bytes, unit = UNITS[unit_index])
    } else {
    format!("{size:.2} {unit}", size = size, unit = UNITS[unit_index])
    }
}

/// アップロード用のパスを生成
pub fn generate_upload_path(base_dir: &Path, category: &str, filename: &str) -> PathBuf {
    let safe_filename = generate_safe_filename(filename);
    let date = chrono::Utc::now();
    let year = date.year();
    let month = date.month();

    base_dir
        .join(category)
        .join(year.to_string())
        .join(format!("{:02}", month))
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
        assert!(safe_name.ends_with(".jpg"));
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
        assert_eq!(format_file_size(1048576), "1.00 MB");
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
