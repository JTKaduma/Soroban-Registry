use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::manifest::{AuditEntry, ExportManifest};

const BUF_SIZE: usize = 65536;

pub fn extract_and_verify(archive_path: &Path, output_dir: &Path) -> Result<ExportManifest> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;

    extract_outer(archive_path, tmp_dir.path())?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    if !manifest_path.exists() || !inner_path.exists() {
        bail!("invalid archive: missing manifest.json or contract.tar.gz");
    }

    let mut manifest: ExportManifest =
        serde_json::from_reader(BufReader::new(File::open(&manifest_path)?))?;

    let computed_hash = compute_sha256_streaming(&inner_path)?;
    if computed_hash != manifest.sha256 {
        bail!(
            "integrity check failed: expected {} got {}",
            manifest.sha256,
            computed_hash
        );
    }

    manifest.audit_trail.push(AuditEntry {
        action: "import_verified".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    fs::create_dir_all(output_dir)?;
    extract_inner(&inner_path, output_dir)?;

    manifest.audit_trail.push(AuditEntry {
        action: "import_extracted".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    Ok(manifest)
}

fn extract_outer(archive_path: &Path, dest: &Path) -> Result<()> {
    let reader = BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?);
    let decoder = GzDecoder::new(reader);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let dest_path = dest.join(&path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = BufWriter::new(File::create(&dest_path)?);
        let mut buf = vec![0u8; BUF_SIZE];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
        }
        out.flush()?;
    }

    Ok(())
}

fn extract_inner(archive_path: &Path, dest: &Path) -> Result<()> {
    let reader = BufReader::with_capacity(BUF_SIZE, File::open(archive_path)?);
    let decoder = GzDecoder::new(reader);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let dest_path = dest.join(&path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = BufWriter::new(File::create(&dest_path)?);
        let mut buf = vec![0u8; BUF_SIZE];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
        }
        out.flush()?;
    }

    Ok(())
}

fn compute_sha256_streaming(path: &Path) -> Result<String> {
    let mut reader = BufReader::with_capacity(BUF_SIZE, File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; BUF_SIZE];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
