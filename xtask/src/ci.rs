//! Conservative classification of an exact pull-request checkout.

use crate::Runner;
use std::path::Path;

/// Validation to run. Unknown evidence always requires full validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Only regular documentation files changed in a verified PR diff.
    Docs,
    /// Run every validation gate.
    Full,
}

impl Mode {
    /// Machine-readable scope consumed by the CI workflow.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Docs => "docs",
            Self::Full => "full",
        }
    }
}

/// Determine validation scope without treating unavailable evidence as docs.
pub fn mode(
    root: &Path,
    runner: &mut Runner<'_>,
    event: &str,
    base: &str,
    head: &str,
) -> Mode {
    if event == "pull_request"
        && sha(base)
        && sha(head)
        && verified_docs(root, runner, base, head).unwrap_or(false)
    {
        return Mode::Docs;
    }
    Mode::Full
}

fn sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn verified_docs(
    root: &Path,
    runner: &mut Runner<'_>,
    base: &str,
    head: &str,
) -> Result<bool, String> {
    if runner("git", &["rev-parse", "HEAD"], root)?.trim() != head {
        return Ok(false);
    }
    let parents =
        runner("git", &["show", "--no-patch", "--format=%P", head], root)?;
    let parents: Vec<_> = parents.split_whitespace().collect();
    if parents.len() != 2 || parents[0] != base || !sha(parents[1]) {
        return Ok(false);
    }
    let raw = runner(
        "git",
        &[
            "diff",
            "--raw",
            "--no-renames",
            "--no-abbrev",
            "--no-ext-diff",
            "--no-textconv",
            "-z",
            base,
            head,
            "--",
        ],
        root,
    )?;
    Ok(docs_diff(&raw))
}

fn docs_diff(raw: &str) -> bool {
    let Some(raw) = raw.strip_suffix('\0').filter(|s| !s.is_empty()) else {
        return false;
    };
    let fields: Vec<_> = raw.split('\0').collect();
    fields.len().is_multiple_of(2)
        && fields.chunks_exact(2).all(|pair| {
            let Some(metadata) = pair[0].strip_prefix(':') else {
                return false;
            };
            let metadata: Vec<_> = metadata.split(' ').collect();
            metadata.len() == 5
                && sha(metadata[2])
                && sha(metadata[3])
                && matches!(
                    (metadata[0], metadata[1], metadata[4]),
                    ("100644", "100644", "M")
                        | ("000000", "100644", "A")
                        | ("100644", "000000", "D")
                )
                && documentation_path(pair[1])
        })
}

fn documentation_path(path: &str) -> bool {
    (matches!(path, "README.md" | "CHANGELOG.md")
        || (path.starts_with("docs/")
            && Path::new(path).extension().is_some_and(|ext| ext == "md")))
        && path.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        })
}
