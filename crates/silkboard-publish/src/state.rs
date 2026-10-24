//! Private atomic counters only: never persist outgoing messages or credentials.

use crate::{
    Error,
    model::{Config, ProgressEvent, Report, bounded},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

fn directory(path: &Path) -> Result<(), Error> {
    let meta = path.symlink_metadata().map_err(|_| Error::PrivateFile)?;
    if !meta.is_dir()
        || !path.is_absolute()
        || path.canonicalize().map_err(|_| Error::PrivateFile)? != path
    {
        return Err(Error::PrivateFile);
    }
    #[cfg(unix)]
    if meta.permissions().mode() & 0o777 != 0o700 {
        return Err(Error::PrivateFile);
    }
    #[cfg(not(unix))]
    return Err(Error::PrivateFile);
    Ok(())
}

fn check(file: &File, path: &Path) -> Result<(), Error> {
    let before = path.symlink_metadata().map_err(|_| Error::PrivateFile)?;
    let actual = file.metadata().map_err(|_| Error::PrivateFile)?;
    if !before.is_file() || !actual.is_file() {
        return Err(Error::PrivateFile);
    }
    #[cfg(unix)]
    if actual.permissions().mode() & 0o777 != 0o600
        || actual.nlink() != 1
        || (before.dev(), before.ino()) != (actual.dev(), actual.ino())
    {
        return Err(Error::PrivateFile);
    }
    Ok(())
}

pub(super) fn read_private(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    directory(path.parent().ok_or(Error::PrivateFile)?)?;
    if !path
        .symlink_metadata()
        .map_err(|_| Error::PrivateFile)?
        .is_file()
    {
        return Err(Error::PrivateFile);
    }
    let file = File::open(path).map_err(|_| Error::PrivateFile)?;
    check(&file, path)?;
    bounded(file, limit).map_err(|_| Error::PrivateFile)
}

fn random_id() -> Result<String, Error> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::State)?;
    Ok(hex(&bytes))
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        write!(s, "{b:02x}").expect("String formatting");
        s
    })
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Counter {
    schema: u8,
    sequence: i64,
    parent: Option<String>,
}

pub(super) fn allocate(
    config: &Config,
    report: Report,
) -> Result<ProgressEvent, Error> {
    directory(&config.state_dir)?;
    let identity = serde_json::to_vec(&(
        &config.endpoint,
        &config.project,
        &report.hostname,
        &report.tool,
        &report.session_id,
        &report.subagent_id,
    ))
    .map_err(|_| Error::Input)?;
    let key = hex(&Sha256::digest(identity));
    let _lock = lock(&config.state_dir, &key)?;
    let path = config.state_dir.join(format!("{key}.json"));
    let previous = match path.symlink_metadata() {
        Ok(_) => serde_json::from_slice::<Counter>(&read_private(&path, 1024)?)
            .map_err(|_| Error::State)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Counter {
            schema: 1,
            sequence: 0,
            parent: report.parent_session_id.clone(),
        },
        Err(_) => return Err(Error::State),
    };
    if previous.schema != 1
        || previous.sequence < 0
        || previous.parent != report.parent_session_id
    {
        return Err(Error::State);
    }
    let sequence = previous.sequence.checked_add(1).ok_or(Error::State)?;
    let event = ProgressEvent {
        event_id: random_id()?,
        sequence,
        report,
    };
    // Refuse encoded requests exceeding the server limit before advancing state.
    if serde_json::to_vec(&event).map_err(|_| Error::Input)?.len() > 8192 {
        return Err(Error::Input);
    }
    replace(
        &path,
        &serde_json::to_vec(&Counter {
            schema: 1,
            sequence,
            parent: event.report.parent_session_id.clone(),
        })
        .map_err(|_| Error::State)?,
    )?;
    Ok(event)
}

/// Keep lock files stable across atomic state replacements.
pub(super) fn lock(directory_path: &Path, key: &str) -> Result<File, Error> {
    directory(directory_path)?;
    let path = directory_path.join(format!("{key}.lock"));
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    options.mode(0o600);
    let file = options.open(&path).map_err(|_| Error::State)?;
    check(&file, &path)?;
    file.try_lock().map_err(|_| Error::Busy)?;
    Ok(file)
}

/// Write private, synchronized state; callers hold the task/session lock.
pub(super) fn replace(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let directory = path.parent().ok_or(Error::State)?;
    let temporary = directory.join(format!(".publisher-{}", random_id()?));
    let persist = || -> Result<(), Error> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temporary).map_err(|_| Error::State)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| Error::State)?;
        fs::rename(&temporary, path).map_err(|_| Error::State)?;
        File::open(directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::State)?;
        Ok(())
    };
    let result = persist();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
