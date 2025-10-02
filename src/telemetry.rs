//! テレメトリ初期化（簡易版）
//!
//! 依存関係の整備が完了するまで、ログ出力と環境フィルタに絞った構成です。
//! 将来的にトレーシングのエクスポータやメトリクスHTTPエンドポイントを再導入します。
//! （詳細は `docs/FEATURES_JA.md` の monitoring セクションを参照）

use std::{
    env,
    error::Error,
    fmt::{self, Display},
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use once_cell::sync::OnceCell;
use tracing_appender::non_blocking;
use tracing_subscriber::{
    EnvFilter,
    fmt::{MakeWriter, format::FmtSpan},
    util::{SubscriberInitExt, TryInitError},
};

const ALREADY_INITIALIZED_MSG: &str = "global default subscriber has already been set";

const DEFAULT_FILTER_DEV_SUFFIX: &str = "tower=info,sqlx=warn,hyper=info,reqwest=warn,tokio=warn";
const DEFAULT_FILTER_PROD_SUFFIX: &str = "tower=warn,sqlx=warn,hyper=warn,reqwest=warn,tokio=warn";

fn load_filter_suffix(env_var: &str, fallback: &str) -> String {
    env::var(env_var).map_or_else(
        |_| fallback.to_string(),
        |path| {
            fs::read_to_string(&path).map_or_else(
                |err| {
                    eprintln!(
                        "WARN: Failed to read {env_var} file '{path}': {err}; using fallback"
                    );
                    fallback.to_string()
                },
                |contents| {
                    let trimmed = contents.trim();
                    if trimmed.is_empty() {
                        eprintln!(
                            "WARN: {env_var} pointed to '{path}' but it was empty; using fallback"
                        );
                        fallback.to_string()
                    } else {
                        trimmed.to_string()
                    }
                },
            )
        },
    )
}

static TELEMETRY_STATE: OnceCell<Arc<TelemetryState>> = OnceCell::new();

#[derive(Debug)]
pub enum TelemetryError {
    InvalidLogFormat(String),
    InvalidEnvFilter(String),
    InvalidLogOutput(String),
    MissingLogOutputPath,
    DirectoryPathProvided(String),
    UnknownLogRotationToken(String),
    TooManyLogOutputTokens,
    UnsupportedLogScheme(String),
    CreateDir { path: String, source: io::Error },
    Init(TryInitError),
    AlreadyInitialized,
}

impl Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLogFormat(value) => {
                write!(f, "Invalid log format string: '{value}'")
            }
            Self::InvalidEnvFilter(value) => {
                write!(f, "Invalid RUST_LOG filter: '{value}'")
            }
            Self::InvalidLogOutput(value) => {
                write!(
                    f,
                    "Invalid LOG_OUTPUT value: '{value}'. Expected formats include 'stdout', 'stderr', or 'file:/path/to/log [rotation=<daily|hourly|never>]'"
                )
            }
            Self::MissingLogOutputPath => {
                write!(
                    f,
                    "LOG_OUTPUT=file:<path> requires a file path, e.g. 'file:/var/log/app.log'"
                )
            }
            Self::DirectoryPathProvided(path) => {
                write!(
                    f,
                    "LOG_OUTPUT path '{path}' must include a file name, e.g. 'file:{path}app.log'"
                )
            }
            Self::UnknownLogRotationToken(token) => {
                write!(
                    f,
                    "Unknown log rotation token '{token}'; expected one of 'daily', 'hourly', or 'never'",
                )
            }
            Self::TooManyLogOutputTokens => {
                write!(f, "Too many tokens supplied to LOG_OUTPUT")
            }
            Self::UnsupportedLogScheme(scheme) => {
                write!(f, "Unsupported LOG_OUTPUT scheme '{scheme}'")
            }
            Self::CreateDir { path, source } => {
                write!(f, "Failed to create directory '{path}': {source}")
            }
            Self::Init(err) => {
                write!(f, "Failed to initialize tracing subscriber: {err}")
            }
            Self::AlreadyInitialized => {
                write!(f, "Telemetry has already been initialized for this process")
            }
        }
    }
}

impl Error for TelemetryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDir { source, .. } => Some(source),
            Self::Init(err) => Some(err),
            _ => None,
        }
    }
}

impl From<TryInitError> for TelemetryError {
    fn from(err: TryInitError) -> Self {
        let message = err.to_string();
        // NOTE: This relies on the `tracing` crate's error message wording to
        // classify already-initialized subscribers. Monitor releases for
        // changes in this string to avoid false negatives.
        if message.contains(ALREADY_INITIALIZED_MSG) {
            Self::AlreadyInitialized
        } else {
            Self::Init(err)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum LogFormat {
    Json,
    Text,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Rotation {
    Never,
    Daily,
    Hourly,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum LogOutput {
    Stdout,
    Stderr,
    File(PathBuf, Rotation),
}

#[derive(Debug)]
struct TelemetryState {
    // Keep the guards alive for the lifetime of the process to ensure
    // buffered log lines are flushed when the worker shuts down.
    _guards: Vec<non_blocking::WorkerGuard>,
    log_output: LogOutput,
}

impl TelemetryState {
    #[allow(clippy::missing_const_for_fn)]
    fn new(guards: Vec<non_blocking::WorkerGuard>, log_output: LogOutput) -> Self {
        Self { _guards: guards, log_output }
    }

    fn flush() {
        log::logger().flush();
    }
}

/// Handle that keeps telemetry resources alive for the duration of the process.
///
/// Dropping a [`TelemetryHandle`] does **not** immediately flush log buffers; the
/// underlying [`non_blocking::WorkerGuard`] instances are owned by the global
/// telemetry state and flush automatically when the process shuts down.
/// The handle primarily exists so callers can detect whether file logging was
/// configured and ensure the guards stay alive for the lifetime of the
/// application.
#[derive(Debug, Clone)]
pub struct TelemetryHandle {
    inner: Arc<TelemetryState>,
}

impl TelemetryHandle {
    #[allow(clippy::missing_const_for_fn)]
    fn from_arc(inner: Arc<TelemetryState>) -> Self {
        Self { inner }
    }

    /// Returns true when logs are written to a file target.
    #[must_use]
    pub fn is_file_logging(&self) -> bool {
        matches!(self.inner.log_output, LogOutput::File(_, _))
    }

    /// Returns the configured log file path when file logging is enabled.
    #[must_use]
    pub fn log_file_path(&self) -> Option<&Path> {
        match &self.inner.log_output {
            LogOutput::File(path, _) => Some(path.as_path()),
            _ => None,
        }
    }

    /// Flushes buffered log records to their underlying sinks.
    ///
    /// This delegates to the global `log` dispatcher and waits for the
    /// non-blocking writer to drain its buffers.
    pub fn flush() {
        TelemetryState::flush();
    }

    #[cfg(test)]
    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Clone)]
enum LogWriter {
    Stdout,
    Stderr,
    File(non_blocking::NonBlocking),
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = Box<dyn Write + Send + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        match self {
            Self::Stdout => Box::new(io::stdout()),
            Self::Stderr => Box::new(io::stderr()),
            Self::File(non_blocking) => Box::new(non_blocking.clone()),
        }
    }
}

/// テレメトリを初期化します。
///
/// `verbose` が `true` の場合、デフォルトのログフィルタをより詳細な設定に切り替えます。
/// `init_telemetry` はグローバルなサブスクライバを一度だけ設定し、以降の呼び出しでは
/// 初回の初期化結果（[`TelemetryHandle`]）を再利用します。
///
/// # Errors
///
/// 環境変数の解析やサブスクライバの構築に失敗した場合、[`TelemetryError`] を返します。
pub fn init_telemetry(verbose: bool) -> Result<TelemetryHandle, TelemetryError> {
    let state = TELEMETRY_STATE.get_or_try_init(|| {
        install_telemetry(verbose).map(Arc::new)
    })?;

    Ok(TelemetryHandle::from_arc(Arc::clone(state)))
}

fn install_telemetry(verbose: bool) -> Result<TelemetryState, TelemetryError> {
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());

    let env_filter = determine_env_filter(verbose, &app_env)?;
    let log_format = determine_log_format(&app_env)?;
    let log_output = determine_log_output(&app_env)?;

    let (writer, guards) = setup_writer(&log_output)?;

    let make_builder = |writer: LogWriter| {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter.clone())
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(writer)
    };

    match log_format {
        LogFormat::Json => {
            make_builder(writer)
                .json()
                .with_ansi(false)
                .finish()
                .try_init()?;
        }
        LogFormat::Text => {
            let use_ansi = match &log_output {
                LogOutput::Stdout => {
                    io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none()
                }
                LogOutput::Stderr => {
                    io::stderr().is_terminal() && env::var_os("NO_COLOR").is_none()
                }
                LogOutput::File(_, _) => false,
            };

            make_builder(writer)
                .compact()
                .with_ansi(use_ansi)
                .finish()
                .try_init()?;
        }
    }

    #[cfg(feature = "monitoring")]
    init_metrics();

    Ok(TelemetryState::new(guards, log_output))
}

fn setup_writer(
    log_output: &LogOutput,
) -> Result<(LogWriter, Vec<non_blocking::WorkerGuard>), TelemetryError> {
    let mut guards = Vec::new();

    let writer = match log_output {
        LogOutput::Stdout => LogWriter::Stdout,
        LogOutput::Stderr => LogWriter::Stderr,
        LogOutput::File(path, rotation) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| TelemetryError::CreateDir {
                    path: parent.to_string_lossy().to_string(),
                    source,
                })?;
            }

            let dir = path.parent().unwrap_or_else(|| Path::new("."));
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .ok_or_else(|| TelemetryError::DirectoryPathProvided(path.display().to_string()))?;

            let appender = match rotation {
                Rotation::Never => tracing_appender::rolling::never(dir, &file_name),
                Rotation::Daily => tracing_appender::rolling::daily(dir, &file_name),
                Rotation::Hourly => tracing_appender::rolling::hourly(dir, &file_name),
            };

            let (non_blocking, guard) = non_blocking(appender);
            guards.push(guard);
            LogWriter::File(non_blocking)
        }
    };

    Ok((writer, guards))
}

fn determine_env_filter(verbose: bool, app_env: &str) -> Result<EnvFilter, TelemetryError> {
    if env::var_os("RUST_LOG").is_some() {
        return EnvFilter::try_from_default_env()
            .map_err(|err| TelemetryError::InvalidEnvFilter(err.to_string()));
    }

    if let Ok(default_from_env) = env::var("RUST_LOG_DEFAULT") {
        return EnvFilter::try_new(default_from_env)
            .map_err(|err| TelemetryError::InvalidEnvFilter(err.to_string()));
    }

    let crate_name = env!("CARGO_PKG_NAME");
    let dev_suffix = load_filter_suffix("LOG_FILTER_DEV_SUFFIX_FILE", DEFAULT_FILTER_DEV_SUFFIX);
    let prod_suffix =
        load_filter_suffix("LOG_FILTER_PROD_SUFFIX_FILE", DEFAULT_FILTER_PROD_SUFFIX);

    let default_filter = if verbose || app_env.eq_ignore_ascii_case("development") {
        format!("debug,{crate_name}=debug,{dev_suffix}")
    } else {
        format!("info,{crate_name}=info,{prod_suffix}")
    };

    Ok(EnvFilter::new(default_filter))
}

fn determine_log_format(app_env: &str) -> Result<LogFormat, TelemetryError> {
    if let Ok(format_env) = env::var("LOG_FORMAT") {
        return format_env.parse();
    }

    if let Ok(default_format) = env::var("LOG_FORMAT_DEFAULT") {
        return default_format.parse();
    }

    match app_env.to_lowercase().as_str() {
        "prod" | "production" | "staging" => Ok(LogFormat::Json),
        "dev" | "development" | "debug" | "local" | "test" => Ok(LogFormat::Text),
        other => {
            eprintln!(
                "WARN: Unrecognized APP_ENV '{other}'; falling back to json log format"
            );
            Ok(LogFormat::Json)
        }
    }
}

fn determine_log_output(app_env: &str) -> Result<LogOutput, TelemetryError> {
    env::var("LOG_OUTPUT").map_or_else(
        |_| {
            if app_env.eq_ignore_ascii_case("production") || app_env.eq_ignore_ascii_case("staging") {
                Ok(LogOutput::Stderr)
            } else {
                Ok(LogOutput::Stdout)
            }
        },
        |value| value.parse(),
    )
}

impl FromStr for LogFormat {
    type Err = TelemetryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            other => Err(TelemetryError::InvalidLogFormat(other.to_string())),
        }
    }
}

impl FromStr for LogOutput {
    type Err = TelemetryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("stdout") {
            return Ok(Self::Stdout);
        }

        if s.eq_ignore_ascii_case("stderr") {
            return Ok(Self::Stderr);
        }

        if let Some((scheme, remainder)) = s.split_once(':') {
            if !scheme.eq_ignore_ascii_case("file") {
                return Err(TelemetryError::UnsupportedLogScheme(scheme.to_string()));
            }

            let rest = remainder.trim();
            if rest.is_empty() {
                return Err(TelemetryError::MissingLogOutputPath);
            }

            let mut parts = rest.split_whitespace();
            let path_str = parts.next().unwrap();
            if path_str.ends_with('/') {
                return Err(TelemetryError::DirectoryPathProvided(path_str.to_string()));
            }

            let path = PathBuf::from(path_str);
            if path.file_name().is_none() {
                return Err(TelemetryError::DirectoryPathProvided(path_str.to_string()));
            }

            let mut rotation = Rotation::Never;
            let mut saw_positional_rotation = false;

            for token in parts {
                if let Some((key, value)) = token.split_once('=') {
                    if key.eq_ignore_ascii_case("rotation") {
                        rotation = parse_rotation_value(value)?;
                    } else {
                        eprintln!(
                            "WARN: Unsupported LOG_OUTPUT option '{key}'; ignoring value '{value}'"
                        );
                    }
                } else {
                    if saw_positional_rotation {
                        return Err(TelemetryError::TooManyLogOutputTokens);
                    }
                    rotation = parse_rotation_value(token)?;
                    saw_positional_rotation = true;
                }
            }

            return Ok(Self::File(path, rotation));
        }

        Err(TelemetryError::InvalidLogOutput(s.to_string()))
    }
}

fn parse_rotation_value(token: &str) -> Result<Rotation, TelemetryError> {
    if token.eq_ignore_ascii_case("daily") {
        Ok(Rotation::Daily)
    } else if token.eq_ignore_ascii_case("hourly") {
        Ok(Rotation::Hourly)
    } else if token.eq_ignore_ascii_case("never") {
        Ok(Rotation::Never)
    } else {
        Err(TelemetryError::UnknownLogRotationToken(token.to_string()))
    }
}

/// Initialize telemetry with optional verbose mode.
///
/// # Deprecated
///
/// ⚠️ ファイル出力を選択した場合、この関数は [`TelemetryHandle`] を返さないため flush の保証がありません。
/// ファイルロギングを利用する場合は [`init_telemetry`] を直接呼び出してください。
#[deprecated(since = "2.1.0", note = "Please use `init_telemetry(verbose)` instead")]
#[doc(hidden)]
pub fn init_telemetry_with_verbose(verbose: bool) -> Result<(), TelemetryError> {
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());

    if matches!(determine_log_output(&app_env), Ok(LogOutput::File(_, _))) {
        tracing::warn!(
            "init_telemetry_with_verbose does not retain WorkerGuard; file flush is not guaranteed. Use init_telemetry(verbose) instead.",
        );
    }

    init_telemetry(verbose).map(|_| ())
}

#[cfg(feature = "monitoring")]
fn init_metrics() {
    // TODO(monitoring): Reintroduce metrics/exporter plumbing when ready.
    tracing::info!("Metrics initialization feature is enabled.");
}

#[cfg(feature = "monitoring")]
/// Gracefully shutdown telemetry systems
#[allow(clippy::missing_const_for_fn)]
pub fn shutdown_telemetry() {
    // TODO(monitoring): Gracefully drain metric/exporter resources once implemented.
    // No-op for now. Dropping WorkerGuards will flush buffers when the process exits.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
        thread,
        time::{Duration, Instant},
    };
    #[allow(clippy::single_match_else)]
    fn read_until_contains(path: &Path, needle: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last_contents = String::new();

        loop {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    if contents.contains(needle) {
                        return contents;
                    }
                    last_contents = contents;
                }
                Err(_) => {
                    // File may not exist yet; retry until deadline.
                }
            }

            #[allow(clippy::manual_assert)]
            if Instant::now() >= deadline {
                panic!("expected to find '{needle}' in {path:?} but only saw: {last_contents}");
            }

            thread::sleep(Duration::from_millis(20));
        }
    }


    use serial_test::serial;
    use temp_env::with_var;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_concurrent_init_reuses_single_state() {
        if TELEMETRY_STATE.get().is_some() {
            // Another test has already initialized telemetry; skip to avoid flakiness.
            return;
        }

        with_var("RUST_LOG", None as Option<&str>, || {
            with_var("LOG_OUTPUT", Some("stdout"), || {
                let threads: Vec<_> = (0..8)
                    .map(|_| thread::spawn(|| init_telemetry(false).expect("thread init")))
                    .collect();

                let baseline = init_telemetry(false).expect("main init");

                for handle in threads {
                    let thread_handle = handle.join().expect("join thread");
                    assert!(thread_handle.ptr_eq(&baseline));
                }
            });
        });
    }

    #[test]
    #[serial]
    fn test_determine_log_format_defaults() {
        with_var("LOG_FORMAT", None as Option<&str>, || {
            with_var("LOG_FORMAT_DEFAULT", None as Option<&str>, || {
                assert_eq!(determine_log_format("production").unwrap(), LogFormat::Json);
                assert_eq!(determine_log_format("staging").unwrap(), LogFormat::Json);
                assert_eq!(
                    determine_log_format("development").unwrap(),
                    LogFormat::Text
                );
            });
        });
    }

    #[test]
    #[serial]
    fn test_determine_log_format_default_override() {
        with_var("LOG_FORMAT", None as Option<&str>, || {
            with_var("LOG_FORMAT_DEFAULT", Some("json"), || {
                assert_eq!(
                    determine_log_format("development").unwrap(),
                    LogFormat::Json
                );
            });

            with_var("LOG_FORMAT_DEFAULT", Some("text"), || {
                assert_eq!(determine_log_format("production").unwrap(), LogFormat::Text);
            });

            with_var("LOG_FORMAT_DEFAULT", Some("invalid"), || {
                let err = determine_log_format("development").expect_err("invalid format");
                assert!(
                    matches!(err, TelemetryError::InvalidLogFormat(value) if value == "invalid")
                );
            });
        });
    }

    #[test]
    fn test_determine_log_format_dev_aliases_and_unknown() {
        assert_eq!(determine_log_format("dev").unwrap(), LogFormat::Text);
        assert_eq!(determine_log_format("LOCAL").unwrap(), LogFormat::Text);
        assert_eq!(determine_log_format("debug").unwrap(), LogFormat::Text);
        assert_eq!(determine_log_format("test").unwrap(), LogFormat::Text);

    // Unknown values now fall back to JSON format while warning via stderr.
    assert_eq!(determine_log_format("qa").unwrap(), LogFormat::Json);
    }

    #[test]
    #[serial]
    fn test_determine_env_filter_defaults() {
        with_var("RUST_LOG", None as Option<&str>, || {
            let f = determine_env_filter(false, "production").expect("should create filter");
            let s = f.to_string();
            assert!(s.contains("info"));
            assert!(s.contains("reqwest=warn"));
            assert!(s.contains("tokio=warn"));

            let f2 = determine_env_filter(true, "development").expect("should create filter");
            let s2 = f2.to_string();
            assert!(s2.contains("debug"));
            assert!(s2.contains("reqwest=warn"));
            assert!(s2.contains("tokio=warn"));
        });
    }

    #[test]
    #[serial]
    fn test_determine_env_filter_prod_alias() {
        with_var("RUST_LOG", None as Option<&str>, || {
            let filter = determine_env_filter(false, "prod").expect("prod alias should work");
            let rendered = filter.to_string();
            assert!(rendered.contains("info"));
        });
    }

    #[test]
    #[serial]
    fn test_determine_env_filter_suffix_override_from_file() {
        with_var("RUST_LOG", None as Option<&str>, || {
            let mut dev_file = tempfile::NamedTempFile::new().expect("dev suffix file");
            writeln!(dev_file, "tower=debug,sqlx=info").expect("write dev suffix");

            let mut prod_file = tempfile::NamedTempFile::new().expect("prod suffix file");
            writeln!(prod_file, "tower=error,sqlx=error").expect("write prod suffix");

            with_var(
                "LOG_FILTER_DEV_SUFFIX_FILE",
                dev_file.path().to_str(),
                || {
                    with_var(
                        "LOG_FILTER_PROD_SUFFIX_FILE",
                        prod_file.path().to_str(),
                        || {
                            let dev_filter =
                                determine_env_filter(true, "development").expect("dev filter");
                            let dev_rendered = dev_filter.to_string();
                            assert!(dev_rendered.contains("tower=debug"));
                            assert!(dev_rendered.contains("sqlx=info"));

                            let prod_filter =
                                determine_env_filter(false, "production").expect("prod filter");
                            let prod_rendered = prod_filter.to_string();
                            assert!(prod_rendered.contains("tower=error"));
                            assert!(prod_rendered.contains("sqlx=error"));
                        },
                    );
                },
            );
        });
    }

    #[test]
    fn test_determine_log_output_parse() {
        assert_eq!("stdout".parse::<LogOutput>().unwrap(), LogOutput::Stdout);
        assert_eq!("stderr".parse::<LogOutput>().unwrap(), LogOutput::Stderr);

        let f = "file:/tmp/app.log".parse::<LogOutput>().unwrap();
        match f {
            LogOutput::File(path, rotation) => {
                assert_eq!(path, PathBuf::from("/tmp/app.log"));
                assert_eq!(rotation, Rotation::Never);
            }
            _ => panic!("expected file"),
        }

        let f = "file:/var/log/app.log daily".parse::<LogOutput>().unwrap();
        match f {
            LogOutput::File(path, rotation) => {
                assert_eq!(path, PathBuf::from("/var/log/app.log"));
                assert_eq!(rotation, Rotation::Daily);
            }
            _ => panic!("expected file"),
        }

        let f = "file:./rel/app.log hourly".parse::<LogOutput>().unwrap();
        match f {
            LogOutput::File(path, rotation) => {
                assert_eq!(path, PathBuf::from("./rel/app.log"));
                assert_eq!(rotation, Rotation::Hourly);
            }
            _ => panic!("expected file"),
        }

        let f = "file:/tmp/keyvalue.log rotation=daily".parse::<LogOutput>().unwrap();
        match f {
            LogOutput::File(path, rotation) => {
                assert_eq!(path, PathBuf::from("/tmp/keyvalue.log"));
                assert_eq!(rotation, Rotation::Daily);
            }
            _ => panic!("expected file"),
        }

        let f = "file:/tmp/unknown.log foo=bar".parse::<LogOutput>().unwrap();
        match f {
            LogOutput::File(path, rotation) => {
                assert_eq!(path, PathBuf::from("/tmp/unknown.log"));
                assert_eq!(rotation, Rotation::Never);
            }
            _ => panic!("expected file"),
        }
    }

    #[test]
    fn test_determine_log_output_invalid() {
        assert!(matches!(
            "file:".parse::<LogOutput>(),
            Err(TelemetryError::MissingLogOutputPath)
        ));
        assert!(matches!(
            "file:/var/log/".parse::<LogOutput>(),
            Err(TelemetryError::DirectoryPathProvided(_))
        ));
        assert!(matches!(
            "file:/tmp/a.log weekly".parse::<LogOutput>(),
            Err(TelemetryError::UnknownLogRotationToken(token)) if token == "weekly"
        ));
        assert!(matches!(
            "file:/tmp/a.log daily extra".parse::<LogOutput>(),
            Err(TelemetryError::TooManyLogOutputTokens)
        ));
        assert!(matches!(
            "udp:/tmp/a.log".parse::<LogOutput>(),
            Err(TelemetryError::UnsupportedLogScheme(scheme)) if scheme == "udp"
        ));
    }

    #[test]
    fn test_setup_writer_creates_directories() {
        let temp = tempdir().expect("temp dir");
        let log_path = temp.path().join("logs").join("app.log");
        assert!(!log_path.parent().unwrap().exists());

        let output = LogOutput::File(log_path.clone(), Rotation::Never);
        let (_writer, guards) = setup_writer(&output).expect("writer setup");
        assert_eq!(guards.len(), 1);
        assert!(log_path.parent().unwrap().exists());
        drop(guards);
    }

    #[test]
    fn test_setup_writer_supports_rotation_tokens() {
        let temp = tempdir().expect("temp dir");
        let log_path = temp.path().join("rotating").join("app.log");

        let parsed: LogOutput = format!("file:{} daily", log_path.to_string_lossy())
            .parse()
            .expect("parse log output");

        match parsed {
            LogOutput::File(path, Rotation::Daily) => {
                assert_eq!(path, log_path);
                #[allow(clippy::redundant_clone)]
                let (_writer, guards) =
                    setup_writer(&LogOutput::File(path.clone(), Rotation::Daily))
                        .expect("setup writer");
                assert_eq!(guards.len(), 1);
                drop(guards);
            }
            other => panic!("unexpected log output: {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn test_init_telemetry_respects_rust_log_default_and_blocks_reinit() {
        with_var("RUST_LOG", None as Option<&str>, || {
            with_var("APP_ENV", Some("development"), || {
                with_var("RUST_LOG_DEFAULT", Some("warn"), || {
                    let temp = tempdir().expect("temp dir");
                    let log_path = temp.path().join("out.log");
                    let output_var = format!("file:{}", log_path.to_string_lossy());

                    with_var("LOG_OUTPUT", Some(output_var.as_str()), || {
                        let handle = init_telemetry(false).expect("first init succeeds");
                        tracing::warn!("visible warning");
                        tracing::info!("muted info");

                        TelemetryHandle::flush();

                        let contents = read_until_contains(&log_path, "visible warning");
                        assert!(contents.contains("visible warning"));
                        assert!(!contents.contains("muted info"));

                        let second =
                            init_telemetry(false).expect("second init should reuse handle");
                        TelemetryHandle::flush();
                        assert!(handle.ptr_eq(&second));
                        #[allow(clippy::redundant_closure_for_method_calls)]
                        {
                            assert_eq!(
                                second.log_file_path().map(|p| p.to_path_buf()),
                                Some(log_path.clone())
                            );
                        }
                    });
                });
            });
        });
    }

    #[test]
    #[serial]
    #[allow(clippy::single_match_else)]
    fn test_init_telemetry_invalid_log_format_bubbles_error() {
        with_var("LOG_FORMAT", Some("invalid"), || {
            match TELEMETRY_STATE.get() {
                Some(_) => {
                    init_telemetry(false).expect("should reuse existing telemetry");
                }
                None => {
                    let err = init_telemetry(false).expect_err("invalid format should error");
                    assert!(
                        matches!(err, TelemetryError::InvalidLogFormat(value) if value == "invalid")
                    );
                }
            }
        });
    }

    #[test]
    #[serial]
    fn test_init_telemetry_invalid_log_output_bubbles_error() {
        with_var("LOG_OUTPUT", Some("file:"), || {
            match TELEMETRY_STATE.get() {
                Some(_) => {
                    init_telemetry(false).expect("should reuse existing telemetry");
                }
                None => {
                    let err = init_telemetry(false).expect_err("invalid output should error");
                    assert!(matches!(err, TelemetryError::MissingLogOutputPath));
                }
            }
        });
    }
}
