use super::{recipes::valid_name, types::*};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionState {
    pub(crate) schema_version: u32,
    pub(crate) scope: String,
    pub(crate) session_id: String,
    pub(crate) revision: String,
    pub(crate) source_revision: String,
    pub(crate) delivered: BTreeMap<String, String>,
}

impl SessionState {
    pub fn revision(&self) -> &str {
        &self.revision
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !valid_name(&self.session_id)
            || self.scope.len() > 256
            || self.revision.len() > 64
            || self.source_revision.len() > 64
            || self.delivered.len() > 128
            || self
                .delivered
                .iter()
                .any(|(id, hash)| id.len() > 64 || hash.len() > 64)
        {
            bail!("Invalid session checkpoint");
        }
        Ok(())
    }
}

/// Explicit local checkpoint for short-lived CLI processes. Stores fingerprints, never source.
/// The parent directory must already exist and should be outside the source repository.
pub fn load_checkpoint(path: &Path) -> Result<Option<SessionState>> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 65536 {
                bail!("Checkpoint must be a regular file no larger than 64 KiB");
            }
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(65537)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 65536 {
                bail!("Checkpoint exceeds 64 KiB");
            }
            let state: SessionState =
                serde_json::from_slice(&bytes).context("Invalid session checkpoint")?;
            state.validate()?;
            Ok(Some(state))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn save_checkpoint(path: &Path, state: &SessionState) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            bail!("Checkpoint destination must be a regular file");
        }
    }
    let mut output = tempfile::NamedTempFile::new_in(parent)?;
    output.write_all(&serde_json::to_vec(state)?)?;
    output.as_file().sync_all()?;
    output.persist(path).map_err(|error| error.error)?;
    Ok(())
}

pub(crate) fn validate_request_session(request: &ContextRequest) -> Result<()> {
    if request.reset_session
        && (request.session_id.is_none() || request.expected_revision.is_some())
    {
        bail!("reset_session requires session_id and no expected_revision");
    }
    if request
        .session_id
        .as_ref()
        .is_some_and(|name| !valid_name(name))
    {
        bail!("session_id must contain 1-64 letters, digits, underscores, or hyphens");
    }
    if request.expected_revision.is_some() && request.session_id.is_none() {
        bail!("expected_revision requires session_id");
    }
    if request
        .expected_revision
        .as_ref()
        .is_some_and(|value| value.len() > 64 || !value.bytes().all(|ch| ch.is_ascii_hexdigit()))
    {
        bail!("Invalid expected_revision");
    }
    Ok(())
}

pub(crate) struct CheckpointLock(std::path::PathBuf);
impl CheckpointLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        let lock = std::path::PathBuf::from(name);
        std::fs::OpenOptions::new().write(true).create_new(true).open(&lock)
            .context("Session checkpoint is locked by another process; remove a stale .lock file only after that process has stopped")?;
        Ok(Self(lock))
    }
}
impl Drop for CheckpointLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
