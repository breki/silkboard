//! Force a valid legacy write between source-admission queries.
use super::*;
use crate::progress::ProgressStore;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let path = std::env::temp_dir().join(format!(
            "silkboard-snapshot-validation-{}",
            crate::secrets::encode(&random)
        ));
        fs::create_dir(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn source_validation_reads_one_revision_during_a_concurrent_commit() {
    let directory = Directory::new();
    let path = directory.0.join("live.sqlite");
    drop(ProgressStore::open(&path).unwrap());
    let writer = Connection::open(&path).unwrap();
    writer.pragma_update(None, "journal_mode", "WAL").unwrap();
    let reader =
        Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let committed = Arc::new(AtomicBool::new(false));
    let observed = committed.clone();
    reader
        .authorizer(Some(move |context: AuthContext<'_>| {
            // The checklist revision has already been read. Commit a valid
            // previous-version edit before the layout queries are prepared.
            if matches!(
                context.action,
                AuthAction::Read {
                    table_name: "todo_layout_schema",
                    ..
                }
            ) && !observed.swap(true, Ordering::SeqCst)
            {
                writer
                    .execute_batch(
                        "BEGIN IMMEDIATE;
                  INSERT INTO todos(text) VALUES('Concurrent task');
                  UPDATE todo_schema SET revision=revision+1;
                  COMMIT;",
                    )
                    .unwrap();
            }
            Authorization::Allow
        }))
        .unwrap();
    let result = validate(&reader);
    assert!(committed.load(Ordering::SeqCst));
    result.expect("a valid concurrent commit must not invalidate admission");
    assert!(
        reader.is_autocommit(),
        "release the read before VACUUM INTO"
    );
    // The new committed state is also valid on its own.
    validate(&reader).unwrap();
    let writer = Connection::open(&path).unwrap();
    writer
        .execute("UPDATE todo_schema SET version=2", [])
        .unwrap();
    assert!(matches!(validate(&reader), Err(ProgressError::NewerSchema)));
    assert!(reader.is_autocommit(), "failed admission releases its read");
}
