//! Operator-only consistent snapshots; restoration never overwrites live state.
use super::{ProgressError, operator::private_options};
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

/// Execute `database backup FILE` or `database restore FILE`.
///
/// `database` is the existing source for backup, or a new destination for restore.
/// All files require private directories and modes. Restore preserves owner
/// passwords/history but revokes all publisher tokens, browser sessions and
/// pending setup links. Only schema 3 is accepted; neither source is migrated.
///
/// # Errors
/// Returns malformed-path/argument, incompatible-schema or storage errors.
pub fn database_command(
    database: &Path,
    args: &[String],
) -> Result<(), ProgressError> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["backup", output] => snapshot(database, Path::new(output), false),
        ["restore", input] => snapshot(Path::new(input), database, true),
        _ => Err(ProgressError::Invalid),
    }
}

fn private(path: &Path, directory: bool) -> Result<(), ProgressError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ProgressError::Unavailable)?;
    if if directory {
        !metadata.is_dir()
    } else {
        !metadata.is_file()
    } {
        return Err(ProgressError::Invalid);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(ProgressError::Invalid);
        }
    }
    Ok(())
}
fn parent(path: &Path) -> Result<&Path, ProgressError> {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(ProgressError::Invalid)
}
fn existing(path: &Path) -> Result<Connection, ProgressError> {
    private(parent(path)?, true)?;
    private(path, false)?;
    let connection = Connection::open_with_flags(
        path.canonicalize()
            .map_err(|_| ProgressError::Unavailable)?,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    connection.busy_timeout(Duration::from_secs(2))?;
    validate(&connection)?;
    Ok(connection)
}
fn validate(connection: &Connection) -> Result<(), ProgressError> {
    let version: i64 =
        connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > 3 {
        return Err(ProgressError::NewerSchema);
    }
    if version != 3 {
        return Err(ProgressError::Invalid);
    }
    let check: String =
        connection.pragma_query_value(None, "quick_check", |r| r.get(0))?;
    if check != "ok"
        || connection.prepare("PRAGMA foreign_key_check")?.exists([])?
    {
        return Err(ProgressError::Invalid);
    }
    // Validate required application tables before creating any destination.
    connection.prepare("SELECT p.token_hash,p.revoked,o.password_hash,s.token_hash,u.closed,u.expires_at,e.payload FROM publishers p,owner o,owner_sessions s,owner_setup u,events e LIMIT 0")?;
    Ok(())
}
fn destination(source: &Path, output: &Path) -> Result<PathBuf, ProgressError> {
    let source = source
        .canonicalize()
        .map_err(|_| ProgressError::Unavailable)?;
    private(parent(output)?, true)?;
    let output = parent(output)?
        .canonicalize()
        .map_err(|_| ProgressError::Unavailable)?
        .join(output.file_name().ok_or(ProgressError::Invalid)?);
    if source.parent() == output.parent() {
        let a = source
            .file_name()
            .ok_or(ProgressError::Invalid)?
            .as_encoded_bytes();
        let b = output
            .file_name()
            .ok_or(ProgressError::Invalid)?
            .as_encoded_bytes();
        if a == b
            || a.strip_prefix(b).is_some_and(|s| s.starts_with(b"-"))
            || b.strip_prefix(a).is_some_and(|s| s.starts_with(b"-"))
        {
            return Err(ProgressError::Invalid);
        }
    }
    reject_companions(&output)?;
    Ok(output)
}

fn reject_companions(output: &Path) -> Result<(), ProgressError> {
    let mut prefix = output
        .file_name()
        .ok_or(ProgressError::Invalid)?
        .to_os_string();
    prefix.push("-");
    for entry in
        fs::read_dir(parent(output)?).map_err(|_| ProgressError::Unavailable)?
    {
        let name = entry.map_err(|_| ProgressError::Unavailable)?.file_name();
        if name
            .as_encoded_bytes()
            .starts_with(prefix.as_encoded_bytes())
        {
            return Err(ProgressError::Invalid);
        }
    }
    Ok(())
}
fn snapshot(
    source: &Path,
    output: &Path,
    restore: bool,
) -> Result<(), ProgressError> {
    let connection = existing(source)?;
    let output = destination(source, output)?;
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|_| ProgressError::Unavailable)?;
    let temporary = parent(&output)?.join(format!(
        ".silkboard-snapshot-{}",
        crate::secrets::encode(&random)
    ));
    let file = private_options()
        .open(&temporary)
        .map_err(|_| ProgressError::Unavailable)?;
    let result = (|| {
        // A reserved empty 0600 file is accepted by VACUUM INTO. Absolute paths
        // avoid URI filename interpretation; bound values avoid SQL injection.
        connection.execute(
            "VACUUM main INTO ?1",
            [temporary.to_str().ok_or(ProgressError::Invalid)?],
        )?;
        let copy = Connection::open_with_flags(
            &temporary,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        validate(&copy)?;
        if restore {
            copy.execute_batch("BEGIN IMMEDIATE; UPDATE publishers SET revoked=1; DELETE FROM owner_sessions; UPDATE owner_setup SET token_hash=NULL,expires_at=0; COMMIT;")?;
        }
        drop(copy);
        file.sync_all().map_err(|_| ProgressError::Unavailable)?;
        // Old journals/WAL files must never be attached to the new database:
        // recovery could otherwise undo the credential revocations above.
        reject_companions(&output)?;
        // Publish only a complete, sanitized snapshot. Hard-link creation is
        // atomic and refuses an existing destination, including symlinks.
        fs::hard_link(&temporary, &output)
            .map_err(|_| ProgressError::Unavailable)?;
        fs::File::open(parent(&output)?)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| ProgressError::Unavailable)?;
        Ok(())
    })();
    let _ = fs::remove_file(&temporary);
    result
}
