use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupManifest {
    schema_version: u32,
    created_at: String,
    state_db_file: String,
    state_db_sha256: String,
    owner_card_file: String,
    owner_card_sha256: String,
}

#[derive(Debug, Clone)]
struct BackupOptions {
    state_db: PathBuf,
    owner_card: PathBuf,
    output_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct RestoreOptions {
    backup_dir: PathBuf,
    state_db: PathBuf,
    owner_card: PathBuf,
    force: bool,
}

#[derive(Debug, Clone)]
struct VerifyOptions {
    backup_dir: PathBuf,
}

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let cmd = args.next().unwrap_or_default();

    match cmd.as_str() {
        "backup" => run_backup(parse_backup_args(args.collect())?),
        "restore" => run_restore(parse_restore_args(args.collect())?),
        "verify" => run_verify(parse_verify_args(args.collect())?),
        _ => {
            print_usage();
            if cmd.is_empty() {
                Ok(())
            } else {
                bail!("unknown command: {cmd}")
            }
        }
    }
}

fn parse_backup_args(args: Vec<String>) -> Result<BackupOptions> {
    let mut state_db = PathBuf::from("runtime/state/vvtv.db");
    let mut owner_card = PathBuf::from("config/owner_card.sample.yaml");
    let mut output_dir = PathBuf::from("runtime/backups");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--state-db" => {
                i += 1;
                state_db = PathBuf::from(require_value(&args, i, "--state-db")?);
            }
            "--owner-card" => {
                i += 1;
                owner_card = PathBuf::from(require_value(&args, i, "--owner-card")?);
            }
            "--output-dir" => {
                i += 1;
                output_dir = PathBuf::from(require_value(&args, i, "--output-dir")?);
            }
            flag => bail!("unknown flag for backup: {flag}"),
        }
        i += 1;
    }

    Ok(BackupOptions {
        state_db,
        owner_card,
        output_dir,
    })
}

fn parse_restore_args(args: Vec<String>) -> Result<RestoreOptions> {
    let mut backup_dir: Option<PathBuf> = None;
    let mut state_db = PathBuf::from("runtime/state/vvtv.db");
    let mut owner_card = PathBuf::from("config/owner_card.sample.yaml");
    let mut force = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--backup-dir" => {
                i += 1;
                backup_dir = Some(PathBuf::from(require_value(&args, i, "--backup-dir")?));
            }
            "--state-db" => {
                i += 1;
                state_db = PathBuf::from(require_value(&args, i, "--state-db")?);
            }
            "--owner-card" => {
                i += 1;
                owner_card = PathBuf::from(require_value(&args, i, "--owner-card")?);
            }
            "--force" => {
                force = true;
            }
            flag => bail!("unknown flag for restore: {flag}"),
        }
        i += 1;
    }

    let backup_dir = backup_dir.ok_or_else(|| anyhow!("--backup-dir is required"))?;
    Ok(RestoreOptions {
        backup_dir,
        state_db,
        owner_card,
        force,
    })
}

fn parse_verify_args(args: Vec<String>) -> Result<VerifyOptions> {
    let mut backup_dir: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--backup-dir" => {
                i += 1;
                backup_dir = Some(PathBuf::from(require_value(&args, i, "--backup-dir")?));
            }
            flag => bail!("unknown flag for verify: {flag}"),
        }
        i += 1;
    }

    Ok(VerifyOptions {
        backup_dir: backup_dir.ok_or_else(|| anyhow!("--backup-dir is required"))?,
    })
}

fn require_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| anyhow!("missing value for {flag}"))
}

fn run_backup(opts: BackupOptions) -> Result<()> {
    if !opts.state_db.exists() {
        bail!("state db not found: {}", opts.state_db.display());
    }
    if !opts.owner_card.exists() {
        bail!("owner card not found: {}", opts.owner_card.display());
    }

    let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let target_dir = opts.output_dir.join(ts);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed creating backup dir {}", target_dir.display()))?;

    let state_copy = target_dir.join("state.db");
    snapshot_sqlite(&opts.state_db, &state_copy)?;

    let owner_copy = target_dir.join("owner_card.yaml");
    fs::copy(&opts.owner_card, &owner_copy).with_context(|| {
        format!(
            "failed copying owner card from {} to {}",
            opts.owner_card.display(),
            owner_copy.display()
        )
    })?;

    let manifest = BackupManifest {
        schema_version: 1,
        created_at: Utc::now().to_rfc3339(),
        state_db_file: "state.db".to_string(),
        state_db_sha256: sha256_file(&state_copy)?,
        owner_card_file: "owner_card.yaml".to_string(),
        owner_card_sha256: sha256_file(&owner_copy)?,
    };

    let manifest_path = target_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("failed writing {}", manifest_path.display()))?;

    println!("backup_dir={}", target_dir.display());
    println!("manifest={}", manifest_path.display());
    Ok(())
}

fn run_restore(opts: RestoreOptions) -> Result<()> {
    let manifest_path = opts.backup_dir.join("manifest.json");
    let manifest: BackupManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed reading {}", manifest_path.display()))?,
    )?;

    if manifest.schema_version != 1 {
        bail!(
            "unsupported manifest schema_version={}",
            manifest.schema_version
        );
    }

    let state_source = opts.backup_dir.join(&manifest.state_db_file);
    let owner_source = opts.backup_dir.join(&manifest.owner_card_file);
    if !state_source.exists() || !owner_source.exists() {
        bail!("backup files missing in {}", opts.backup_dir.display());
    }

    let state_hash = sha256_file(&state_source)?;
    let owner_hash = sha256_file(&owner_source)?;
    if state_hash != manifest.state_db_sha256 {
        bail!("state db checksum mismatch");
    }
    if owner_hash != manifest.owner_card_sha256 {
        bail!("owner card checksum mismatch");
    }

    if !opts.force {
        if opts.state_db.exists() {
            bail!(
                "target state db exists (use --force): {}",
                opts.state_db.display()
            );
        }
        if opts.owner_card.exists() {
            bail!(
                "target owner card exists (use --force): {}",
                opts.owner_card.display()
            );
        }
    }

    replace_file(&state_source, &opts.state_db)?;
    replace_file(&owner_source, &opts.owner_card)?;

    println!("restored_state_db={}", opts.state_db.display());
    println!("restored_owner_card={}", opts.owner_card.display());
    Ok(())
}

fn run_verify(opts: VerifyOptions) -> Result<()> {
    let manifest_path = opts.backup_dir.join("manifest.json");
    let manifest: BackupManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed reading {}", manifest_path.display()))?,
    )?;

    if manifest.schema_version != 1 {
        bail!(
            "unsupported manifest schema_version={}",
            manifest.schema_version
        );
    }

    let state_source = opts.backup_dir.join(&manifest.state_db_file);
    let owner_source = opts.backup_dir.join(&manifest.owner_card_file);
    if !state_source.exists() || !owner_source.exists() {
        bail!("backup files missing in {}", opts.backup_dir.display());
    }

    let state_hash = sha256_file(&state_source)?;
    let owner_hash = sha256_file(&owner_source)?;
    if state_hash != manifest.state_db_sha256 {
        bail!("state db checksum mismatch");
    }
    if owner_hash != manifest.owner_card_sha256 {
        bail!("owner card checksum mismatch");
    }

    println!("backup_verified=true");
    println!("backup_dir={}", opts.backup_dir.display());
    println!("state_db_sha256={state_hash}");
    println!("owner_card_sha256={owner_hash}");
    Ok(())
}

fn snapshot_sqlite(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("failed removing old {}", destination.display()))?;
    }

    let source_conn = Connection::open(source)
        .with_context(|| format!("failed opening sqlite source {}", source.display()))?;

    let destination_sql = destination
        .to_str()
        .ok_or_else(|| anyhow!("invalid UTF-8 path for destination"))?
        .replace('"', "\"\"");

    source_conn
        .execute_batch(&format!("VACUUM INTO \"{destination_sql}\";"))
        .context("sqlite VACUUM INTO failed")?;

    Ok(())
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating parent dir {}", parent.display()))?;
    }

    let tmp = target.with_extension("tmp.restore");
    fs::copy(source, &tmp)
        .with_context(|| format!("failed copying {} to {}", source.display(), tmp.display()))?;

    fs::rename(&tmp, target).with_context(|| format!("failed replacing {}", target.display()))?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed opening {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn print_usage() {
    println!(
        "Usage:\n  vvtv-admin backup [--state-db PATH] [--owner-card PATH] [--output-dir PATH]\n  vvtv-admin restore --backup-dir PATH [--state-db PATH] [--owner-card PATH] [--force]\n  vvtv-admin verify --backup-dir PATH"
    );
}
