#![allow(dead_code)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use biscuit_auth::KeyPair;
use clap::{Parser, ValueEnum};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

mod gen_biscuit_keys_backup;
mod gen_biscuit_keys_manifest;

// Grouped options for outputs to reduce argument count
#[derive(Clone, Copy)]
struct OutputsOptions<'a> {
    format: Option<&'a str>,
    out_dir: Option<&'a str>,
    env_file: Option<&'a str>,
    backup: bool,
    force: bool,
    priv_b64: &'a str,
    pub_b64: &'a str,
}

// Extract version number from filenames like biscuit_private_v12.b64
fn parse_version(name: &str) -> Option<u32> {
    if let Some(idx) = name.rfind("_v") {
        // expect pattern *_v<digits>.b64
        let ver_part = &name[idx + 2..];
        if let Some(dot) = ver_part.find('.') {
            // remove extension
            let digits = &ver_part[..dot];
            return digits.parse().ok();
        }
    }
    None
}

fn next_version(dir: &Path) -> u32 {
    let mut max_v: u32 = 0;
    if dir.exists() {
        if let Ok(read) = fs::read_dir(dir) {
            for entry in read.flatten() {
                let name = entry.file_name();
                if let Some(s) = name.to_str() {
                    if let Some(v) = parse_version(s) {
                        if v > max_v {
                            max_v = v;
                        }
                    }
                }
            }
        }
    }
    max_v + 1
}

fn write_file_if_allowed(path: &Path, data: &str, force: bool) -> std::io::Result<()> {
    if path.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        ));
    }
    let mut f = fs::File::create(path)?;
    f.write_all(data.as_bytes())?;
    Ok(())
}

fn append_env_file(path: &Path, priv_b64: &str, pub_b64: &str, force: bool) -> std::io::Result<()> {
    if !path.exists() {
        create_env_file(path, priv_b64, pub_b64)
    } else {
        let content = fs::read_to_string(path)?;
        if content.contains("BISCUIT_PRIVATE_KEY_B64=")
            || content.contains("BISCUIT_PUBLIC_KEY_B64=")
        {
            replace_env_entries(path, &content, priv_b64, pub_b64, force)
        } else {
            append_env_entries(path, priv_b64, pub_b64)
        }
    }
}

fn create_env_file(path: &Path, priv_b64: &str, pub_b64: &str) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    writeln!(f, "# Generated biscuit keys")?;
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={priv_b64}")?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={pub_b64}")?;
    Ok(())
}
fn replace_env_entries(
    path: &Path,
    content: &str,
    priv_b64: &str,
    pub_b64: &str,
    force: bool,
) -> std::io::Result<()> {
    if !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already contains biscuit entries", path.display()),
        ));
    }
    let filtered = filter_out_biscuit_lines(content);
    write_filtered_env_and_keys(path, &filtered, priv_b64, pub_b64)
}

fn filter_out_biscuit_lines(content: &str) -> Vec<&str> {
    content
        .lines()
    .filter(|line| !(
        line.starts_with("BISCUIT_PRIVATE_KEY_B64=") || line.starts_with("BISCUIT_PUBLIC_KEY_B64=")
    ))
        .collect()
}

fn write_filtered_env_and_keys(
    path: &Path,
    lines: &[&str],
    priv_b64: &str,
    pub_b64: &str,
) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    for line in lines {
        writeln!(f, "{line}")?;
    }
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={priv_b64}")?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={pub_b64}")?;
    Ok(())
}

fn append_env_entries(path: &Path, priv_b64: &str, pub_b64: &str) -> std::io::Result<()> {
    let mut f = fs::OpenOptions::new().append(true).open(path)?;
    writeln!(f, "\n# Generated biscuit keys")?;
    writeln!(f, "BISCUIT_PRIVATE_KEY_B64={priv_b64}")?;
    writeln!(f, "BISCUIT_PUBLIC_KEY_B64={pub_b64}")?;
    Ok(())
}

fn report_write_file_result(path: &Path, res: std::io::Result<()>, label: &str, force: bool) {
    match res {
        Ok(()) => println!("Wrote {label} to {}", path.display()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists && !force {
                eprintln!(
                    "{} already exists at {}. Use --force to overwrite.",
                    label,
                    path.display()
                );
            } else {
                eprintln!("Failed to write {label}: {e}");
            }
        }
    }
}

fn report_env_result(envfile: &str, res: std::io::Result<()>, force: bool) {
    match res {
        Ok(()) => println!("Written keys into {envfile}"),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists && !force {
                eprintln!(
                    "{envfile} already contains biscuit entries. Use --force to overwrite or choose another env file."
                );
            } else {
                eprintln!("Failed to write env file {envfile}: {e}");
            }
        }
    }
}

// --- DRY helpers for output flows ---
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
struct VersionOptions {
    versioned: bool,
    latest_alias: bool,
    no_manifest: bool,
    prune: Option<usize>,
}

struct FilesWriteOptions {
    backup_dir: Option<PathBuf>,
    max_backups: Option<usize>,
    compress_opt: Option<bool>,
    force: bool,
}
fn handle_files_output(ctx: &FilesOutputContext<'_>) -> cms_backend::Result<()> {
    // Delegate the full files output flow (create dir, write files, backups,
    // and finalization) to the manifest helper which centralizes the
    // end-to-end behavior. This keeps the binary thin and reduces local
    // complexity metrics.
    gen_biscuit_keys_manifest::handle_files_output_full(ctx)
}

fn resolve_paths_and_write(
    ctx: &FilesOutputContext<'_>,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let (priv_path, pub_path) = resolve_output_paths(ctx.path, ctx.vopts.versioned);
    // Write files (with optional backups)
    write_files_flow(
        &priv_path,
        &pub_path,
        ctx.backup,
        ctx.options,
        ctx.priv_b64,
        ctx.pub_b64,
    );
    (priv_path, pub_path)
}

fn finalize_versioned_flow(ctx: &FilesOutputContext<'_>, priv_path: &Path, _pub_path: &Path) {
    if ctx.vopts.versioned {
        // Delegate alias/manifest/prune handling to the manifest module which will
        // update manifest.json, prune old versions and also manage the latest alias.
        gen_biscuit_keys_manifest::finalize_versioned(
            ctx.path,
            priv_path,
            ctx.priv_b64,
            ctx.pub_b64,
            ctx.vopts.no_manifest,
            ctx.vopts.prune,
        );
    }
}

/// Ensure the output directory exists; returns Err on failure.
fn create_dir_and_resolve_paths(path: &Path) -> cms_backend::Result<()> {
    if let Err(e) = fs::create_dir_all(path) {
        return Err(cms_backend::AppError::Internal(format!(
            "Failed to create out-dir {}: {}",
            path.display(),
            e
        )));
    }
    Ok(())
}

/// Encapsulate the write + backup flow so the main function stays small.
fn write_files_flow(
    priv_path: &Path,
    pub_path: &Path,
    backup: bool,
    options: &FilesWriteOptions,
    priv_b64: &str,
    pub_b64: &str,
) {
    perform_files_write(priv_path, pub_path, backup, options, priv_b64, pub_b64);
}

/// Handle the versioned-specific post steps (alias/manifest/prune).
fn post_versioned_flow(
    path: &Path,
    vopts: &VersionOptions,
    priv_path: &Path,
    priv_b64: &str,
    pub_b64: &str,
) {
    if vopts.versioned {
        apply_versioned_post(path, vopts, priv_path, priv_b64, pub_b64);
    }
}

struct FilesOutputContext<'a> {
    path: &'a Path,
    vopts: &'a VersionOptions,
    options: &'a FilesWriteOptions,
    backup: bool,
    priv_b64: &'a str,
    pub_b64: &'a str,
}

fn resolve_output_paths(path: &Path, versioned: bool) -> (std::path::PathBuf, std::path::PathBuf) {
    if versioned {
        let v = next_version(path);
        (
            path.join(format!("biscuit_private_v{v}.b64")),
            path.join(format!("biscuit_public_v{v}.b64")),
        )
    } else {
        (
            path.join("biscuit_private.b64"),
            path.join("biscuit_public.b64"),
        )
    }
}

/// Perform the actual file writes for the private/public files and handle backups.
fn perform_files_write(
    priv_path: &Path,
    pub_path: &Path,
    backup: bool,
    options: &FilesWriteOptions,
    priv_b64: &str,
    pub_b64: &str,
) {
    // Attempt backups first (if requested). Errors are non-fatal for the write operation.
    if let Err(e) = gen_biscuit_keys_backup::maybe_backup_file(
        priv_path,
        backup,
        options.backup_dir.as_deref(),
        options.max_backups,
        options.compress_opt,
    ) {
        eprintln!("Backup failed: {e}");
    }
    if let Err(e) = gen_biscuit_keys_backup::maybe_backup_file(
        pub_path,
        backup,
        options.backup_dir.as_deref(),
        options.max_backups,
        options.compress_opt,
    ) {
        eprintln!("Backup failed: {e}");
    }

    // Write private key
    let res_priv = write_file_if_allowed(priv_path, priv_b64, options.force);
    report_write_file_result(priv_path, res_priv, "private key file", options.force);

    // Write public key
    let res_pub = write_file_if_allowed(pub_path, pub_b64, options.force);
    report_write_file_result(pub_path, res_pub, "public key file", options.force);
}

fn apply_versioned_post(
    path: &Path,
    vopts: &VersionOptions,
    priv_path: &Path,
    priv_b64: &str,
    pub_b64: &str,
) {
    // Finalization (manifest update, pruning, alias updates) delegated to manifest
    // module which centralizes this behavior.
    gen_biscuit_keys_manifest::finalize_versioned(
        path,
        priv_path,
        priv_b64,
        pub_b64,
        vopts.no_manifest,
        vopts.prune,
    );
}


#[allow(clippy::too_many_arguments)]
fn handle_env_output(
    envfile: &str,
    backup: bool,
    backup_dir: Option<&Path>,
    max_backups: Option<usize>,
    backup_compress: bool,
    force: bool,
    priv_b64: &str,
    pub_b64: &str,
) {
    let path = Path::new(envfile);
    maybe_backup_env(path, backup, backup_dir, max_backups, backup_compress);
    perform_env_write_and_report(path, priv_b64, pub_b64, force, envfile);
}

fn maybe_backup_env(
    path: &Path,
    backup: bool,
    backup_dir: Option<&Path>,
    max_backups: Option<usize>,
    backup_compress: bool,
) {
    if let Err(e) = gen_biscuit_keys_backup::maybe_backup_file(
        path,
        backup,
        backup_dir,
        max_backups,
        Some(backup_compress),
    ) {
        eprintln!("Backup failed: {e}");
    }
}
fn perform_env_write(
    path: &Path,
    priv_b64: &str,
    pub_b64: &str,
    force: bool,
) -> std::io::Result<()> {
    append_env_file(path, priv_b64, pub_b64, force)
}

fn perform_env_write_and_report(
    path: &Path,
    priv_b64: &str,
    pub_b64: &str,
    force: bool,
    envfile: &str,
) {
    let res = perform_env_write(path, priv_b64, pub_b64, force);
    report_env_result(envfile, res, force);
}
// Backup orchestration is implemented in `gen_biscuit_keys_backup.rs`.
// We call it directly where needed to avoid extra wrapper functions in this
// binary, which keeps this file smaller and simpler.

// alias updates are now handled by the manifest module; no local helper needed.
// Backup related helpers moved to `gen_biscuit_keys_backup.rs` to reduce file size and complexity.
// Thin wrappers delegate to the backup module functions.
// Delegate backup orchestration to the backup module directly. The module
// contains the implementation and helpers; keeping only delegations here
// avoids duplicating complex logic in the binary's main file.
// All call sites in this file call into the module directly now.
// clap-based argument parsing is used; helper suggestion/levenshtein removed.

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Generate biscuit keypair (base64) and optionally write to files/.env"
)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// Write `biscuit_private.b64` and `biscuit_public.b64` into <dir>
    #[arg(long, value_name = "DIR")]
    out_dir: Option<PathBuf>,

    /// Append `BISCUIT_PRIVATE_KEY_B64` / `BISCUIT_PUBLIC_KEY_B64` to <file> (default: none)
    #[arg(long, value_name = "FILE")]
    env_file: Option<PathBuf>,

    /// Explicit output target. If omitted, defaults to previous behavior (use provided targets).
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,

    /// Overwrite existing files / env entries when present
    #[arg(long)]
    force: bool,

    /// Move existing files/env to a timestamped .bak before writing
    #[arg(long)]
    backup: bool,
    /// Directory to store backups (default: same directory as target)
    #[arg(long, value_name = "DIR")]
    backup_dir: Option<PathBuf>,
    /// Maximum number of backups to keep per target (older backups will be removed). 0 means unlimited
    #[arg(long, value_name = "N")]
    max_backups: Option<usize>,
    /// Compress created backups with gzip (removes uncompressed backup file)
    #[arg(long)]
    backup_compress: bool,

    /// Save as versioned files (`biscuit_private_vN.b64`) and keep latest unversioned copy
    #[arg(long)]
    versioned: bool,

    /// List existing versioned keys and exit
    #[arg(long)]
    list: bool,

    /// When used with `--versioned`, also write/update unversioned `biscuit_private.b64` / `biscuit_public.b64` as alias to latest
    #[arg(long)]
    latest_alias: bool,

    /// Keep only the newest N versioned key sets (applies after writing). 0 disables pruning.
    #[arg(long, value_name = "N")]
    prune: Option<usize>,

    /// Skip manifest.json update
    #[arg(long)]
    no_manifest: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Files,
    Env,
    Both,
    Stdout,
}

#[allow(clippy::too_many_lines)]
fn main() -> cms_backend::Result<()> {
    let args = Args::parse();
    let out_dir: Option<String> = args
        .out_dir
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned());
    let env_file: Option<String> = args
        .env_file
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned());
    let format = args.format.map(|f| match f {
        OutputFormat::Files => "files".to_string(),
        OutputFormat::Env => "env".to_string(),
        OutputFormat::Both => "both".to_string(),
        OutputFormat::Stdout => "stdout".to_string(),
    });
    let force = args.force;
    let backup = args.backup;

    // If user requested the list of versions, handle that and exit early
    if args.list {
        handle_list_versions(out_dir.as_deref().unwrap_or("keys"));
        return Ok(());
    }

    let kp = KeyPair::new();
    let priv_b64 = STANDARD.encode(kp.private().to_bytes());
    let pub_b64 = STANDARD.encode(kp.public().to_bytes());

    println!("# Generated biscuit keypair (base64)");
    println!("BISCUIT_PRIVATE_KEY_B64={priv_b64}");
    println!("BISCUIT_PUBLIC_KEY_B64={pub_b64}");

    // Consolidate output options into a struct to simplify downstream calls
    let opts = OutputsOptions {
        format: format.as_deref(),
        out_dir: out_dir.as_deref(),
        env_file: env_file.as_deref(),
        backup,
        force,
        priv_b64: &priv_b64,
        pub_b64: &pub_b64,
    };

    decide_and_perform_outputs(opts, &args)?;
    Ok(())
}

fn decide_and_perform_outputs(opts: OutputsOptions<'_>, args: &Args) -> cms_backend::Result<()> {
    do_perform_outputs(opts, args)?;
    notify_if_no_output(opts.format, opts.out_dir, opts.env_file);
    Ok(())
}

fn do_perform_outputs(opts: OutputsOptions<'_>, args: &Args) -> cms_backend::Result<()> {
    // Normalize format to lowercase before dispatching
    let normalized = opts.format.map(|s| s.to_ascii_lowercase());
    let opts = OutputsOptions {
        format: normalized.as_deref(),
        ..opts
    };
    perform_outputs(opts, args)
}

fn notify_if_no_output(format: Option<&str>, out_dir: Option<&str>, env_file: Option<&str>) {
    if format.is_none() && out_dir.is_none() && env_file.is_none() {
        println!("No output target specified; keys were only printed to stdout.");
    }
}

fn perform_outputs(opts: OutputsOptions<'_>, args: &Args) -> cms_backend::Result<()> {
    if let Some(f) = opts.format {
        perform_outputs_with_format(f, opts, args)?;
    } else {
        perform_outputs_without_format(opts, args)?;
    }
    Ok(())
}

fn perform_outputs_with_format(
    f: &str,
    opts: OutputsOptions<'_>,
    args: &Args,
) -> cms_backend::Result<()> {
    match f {
        "files" => handle_files_for_dir(
            opts.out_dir.unwrap_or("keys"),
            args,
            opts.backup,
            opts.force,
            opts.priv_b64,
            opts.pub_b64,
        )?,
        "env" => {
            let env_path = opts.env_file.unwrap_or(".env");
            handle_env_output(
                env_path,
                opts.backup,
                args.backup_dir.as_deref(),
                args.max_backups,
                args.backup_compress,
                opts.force,
                opts.priv_b64,
                opts.pub_b64,
            );
        }
        "both" => {
            handle_files_for_dir(
                opts.out_dir.unwrap_or("keys"),
                args,
                opts.backup,
                opts.force,
                opts.priv_b64,
                opts.pub_b64,
            )?;
            handle_env_output(
                opts.env_file.unwrap_or(".env"),
                opts.backup,
                args.backup_dir.as_deref(),
                args.max_backups,
                args.backup_compress,
                opts.force,
                opts.priv_b64,
                opts.pub_b64,
            );
        }
        _ => {}
    }
    Ok(())
}

fn perform_outputs_without_format(
    opts: OutputsOptions<'_>,
    args: &Args,
) -> cms_backend::Result<()> {
    if let Some(dir) = opts.out_dir {
        handle_files_for_dir(
            dir,
            args,
            opts.backup,
            opts.force,
            opts.priv_b64,
            opts.pub_b64,
        )?;
    }
    if let Some(envfile) = opts.env_file {
        handle_env_output(
            envfile,
            opts.backup,
            args.backup_dir.as_deref(),
            args.max_backups,
            args.backup_compress,
            opts.force,
            opts.priv_b64,
            opts.pub_b64,
        );
    }
    Ok(())
}

fn make_files_options(args: &Args, force: bool) -> FilesWriteOptions {
    FilesWriteOptions {
        backup_dir: args.backup_dir.clone(),
        max_backups: args.max_backups,
        compress_opt: Some(args.backup_compress),
        force,
    }
}

fn make_version_options(args: &Args) -> VersionOptions {
    VersionOptions {
        versioned: args.versioned,
        latest_alias: args.latest_alias,
        no_manifest: args.no_manifest,
        prune: args.prune,
    }
}

fn handle_files_for_dir(
    dir: &str,
    args: &Args,
    backup: bool,
    force: bool,
    priv_b64: &str,
    pub_b64: &str,
) -> cms_backend::Result<()> {
    let options = make_files_options(args, force);
    let vopts = make_version_options(args);
    let ctx = FilesOutputContext {
        path: Path::new(dir),
        vopts: &vopts,
        options: &options,
        backup,
        priv_b64,
        pub_b64,
    };
    handle_files_output(&ctx)
}

fn handle_list_versions(dir: &str) {
    let path = Path::new(dir);
    match fs::read_dir(path) {
        Ok(read) => {
            let mut versions: Vec<(u32, String)> = Vec::new();
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some(v) = parse_version(&name) {
                    versions.push((v, name));
                }
            }
            if versions.is_empty() {
                println!("No versioned keys found in {}", path.display());
            } else {
                versions.sort_by_key(|x| x.0);
                println!("Found versions (oldest -> newest):");
                for (v, name) in versions {
                    println!("v{v} => {name}");
                }
            }
        }
        Err(e) => eprintln!("Cannot read directory {}: {e}", path.display()),
    }
}
