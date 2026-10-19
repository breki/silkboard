//! Local operator operations. Secrets are written only to new private files.

use super::{ProgressError, ProgressStore};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub(super) fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

pub(super) fn prepare_database(path: &Path) -> Result<(), ProgressError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| ProgressError::Unavailable)?;
    if !metadata.is_dir() {
        return Err(ProgressError::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 {
            return Err(ProgressError::Invalid);
        }
    }
    match private_options().open(path) {
        Ok(file) => file.sync_all().map_err(|_| ProgressError::Unavailable)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| ProgressError::Unavailable)?;
            if !metadata.is_file() {
                return Err(ProgressError::Invalid);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.mode() & 0o077 != 0 {
                    return Err(ProgressError::Invalid);
                }
            }
        }
        Err(_) => return Err(ProgressError::Unavailable),
    }
    Ok(())
}

/// Execute `publisher issue PROJECT LABEL TOKEN_FILE` or
/// `publisher revoke PROJECT LABEL`, against an explicitly selected database.
///
/// Never accepts a token in arguments or prints one. Issue refuses to overwrite
/// files or collide with database/sidecar paths. Failures after token creation
/// attempt revocation before returning an error.
///
/// # Errors
/// Returns malformed-argument, credential-conflict, file or database errors.
pub fn publisher_command(
    database: &Path,
    args: &[String],
) -> Result<(), ProgressError> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["issue", project, label, file] => {
            let store = ProgressStore::open(database)?;
            reject_storage_collision(database, Path::new(file))?;
            let mut output = private_options()
                .open(file)
                .map_err(|_| ProgressError::Unavailable)?;
            let result = (|| {
                let token = store.issue_token(project, label)?;
                if writeln!(output, "{token}")
                    .and_then(|()| output.sync_all())
                    .is_err()
                {
                    store.revoke_token(project, label)?;
                    return Err(ProgressError::Unavailable);
                }
                Ok(())
            })();
            if result.is_err() {
                let _ = fs::remove_file(file);
            }
            result
        }
        ["revoke", project, label] => {
            ProgressStore::open(database)?.revoke_token(project, label)
        }
        _ => Err(ProgressError::Invalid),
    }
}

fn reject_storage_collision(
    database: &Path,
    output: &Path,
) -> Result<(), ProgressError> {
    let database = database
        .canonicalize()
        .map_err(|_| ProgressError::Unavailable)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|_| ProgressError::Unavailable)?;
    let name = output.file_name().ok_or(ProgressError::Invalid)?;
    if database.parent() == Some(parent.as_path()) {
        let database_name =
            database.file_name().ok_or(ProgressError::Invalid)?;
        let mut sidecar_prefix = database_name.to_os_string();
        sidecar_prefix.push("-");
        if name == database_name
            || name
                .as_encoded_bytes()
                .starts_with(sidecar_prefix.as_encoded_bytes())
        {
            return Err(ProgressError::Invalid);
        }
    }
    Ok(())
}
