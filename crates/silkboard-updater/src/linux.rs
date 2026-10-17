//! Linux deployment boundaries with injectable command execution.

use crate::{
    Backend, Release, State, Version, evidence, outcome::Admission,
    profile::Policy, storage,
};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

const FETCH: &str = "/var/lib/silkboard-fetch/repo.git";
const TOOLS: &str = "/usr/local/libexec/silkboard";

/// Installed updater paths and a command runner; no release-controlled program
/// is invoked with root privileges.
pub struct Linux<R> {
    runner: R,
    policy: Policy,
    state: PathBuf,
    releases: PathBuf,
    current: PathBuf,
}

#[cfg(test)]
mod tests;

impl<R> Linux<R>
where
    R: FnMut(&str, &[&str]) -> Result<String, String>,
{
    /// Use the production guest's fixed operator-owned paths.
    pub fn production(runner: R) -> Self {
        Self {
            runner,
            policy: Policy::silkboard(),
            state: "/var/lib/silkboard-updater/state.json".into(),
            releases: "/opt/silkboard/releases".into(),
            current: "/opt/silkboard/current".into(),
        }
    }

    /// Bind validated admission policy to the installed Silkboard application adapter.
    /// Other applications must supply their own reviewed backend implementation.
    ///
    /// # Errors
    /// Rejects an application without a registered adapter.
    pub fn configured(runner: R, policy: Policy) -> Result<Self, String> {
        if policy.application() != "silkboard" {
            return Err("application adapter is not registered".into());
        }
        let mut backend = Self::production(runner);
        backend.policy = policy;
        Ok(backend)
    }

    fn run(&mut self, program: &str, args: &[&str]) -> Result<String, String> {
        (self.runner)(program, args)
    }

    fn fetch_git(&mut self, args: &[&str]) -> Result<String, String> {
        let mut command = vec![
            "180",
            "runuser",
            "-u",
            "silkfetch",
            "--",
            "env",
            "-i",
            "HOME=/var/lib/silkboard-fetch",
            "PATH=/usr/bin:/bin",
            "GIT_TERMINAL_PROMPT=0",
            "GIT_CONFIG_NOSYSTEM=1",
            "git",
            "--git-dir",
            FETCH,
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "credential.helper=",
            "-c",
            "credential.helper=/usr/local/libexec/silkboard/git-credential",
        ];
        command.extend_from_slice(args);
        self.run("timeout", &command)
    }

    fn api(&mut self, path: &str) -> Result<Value, String> {
        let repository = self.policy.repository().to_owned();
        let output = self.run(
            "runuser",
            &[
                "-u",
                "silkfetch",
                "--",
                "/usr/local/libexec/silkboard/github-read",
                &repository,
                path,
            ],
        )?;
        serde_json::from_str(&output).map_err(|e| format!("GitHub JSON: {e}"))
    }

    fn evidence(&mut self, release: &Release) -> Result<Admission, String> {
        let workflow_id = self.policy.workflow_id();
        let workflow = self.api(&format!("actions/workflows/{workflow_id}"))?;
        if !evidence::valid_workflow_for(&workflow, &self.policy) {
            return Err("registered release workflow identity changed".into());
        }
        let runs = self.api(&format!("actions/workflows/{workflow_id}/runs?event=push&branch={}&head_sha={}&per_page=100", release.tag(), release.commit()))?;
        let Some(id) = evidence::passing_run_for(&runs, release, &self.policy)
        else {
            return Ok(classify_incomplete(&runs));
        };
        let run = self.api(&format!("actions/runs/{id}"))?;
        if !evidence::valid_run_for(&run, release, &self.policy) {
            return Ok(incomplete_run(&run));
        }
        let attempt =
            run["run_attempt"].as_u64().ok_or("missing run attempt")?;
        let jobs = self.api(&format!(
            "actions/runs/{id}/attempts/{attempt}/jobs?per_page=100"
        ))?;
        if !evidence::passing_jobs_for(&jobs, &self.policy) {
            return Ok(Admission::Rejected {
                reason: "required jobs did not all pass".into(),
            });
        }
        let after = self.api(&format!("actions/runs/{id}"))?;
        if !evidence::valid_run_for(&after, release, &self.policy)
            || after["run_attempt"] != attempt
        {
            return Ok(incomplete_run(&after));
        }
        eprintln!(
            "Release suite passed: tag={} commit={} workflow={workflow_id} run={id} attempt={attempt}",
            release.tag(),
            release.commit()
        );
        Ok(Admission::Eligible {
            release: release.clone(),
        })
    }

    fn directory(&self, release: &Release) -> PathBuf {
        self.releases
            .join(format!("{}-{}", release.tag(), release.commit()))
    }

    fn identity(&self, directory: &Path) -> Result<Release, String> {
        let identity = fs::read_to_string(directory.join("identity"))
            .map_err(|e| e.to_string())?;
        let lines: Vec<_> = identity.lines().collect();
        if lines.len() != 2 {
            return Err("invalid installed identity".into());
        }
        let release = Release::new(lines[0], lines[1])?;
        if self.directory(&release) != directory {
            return Err("release path differs from identity".into());
        }
        Ok(release)
    }
}

impl<R> Backend for Linux<R>
where
    R: FnMut(&str, &[&str]) -> Result<String, String>,
{
    fn load(&mut self) -> Result<State, String> {
        match fs::read(&self.state) {
            Ok(bytes) => storage::decode(
                &bytes,
                &self.policy,
                storage::SILKBOARD_ADAPTER,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(State::default())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn save(&mut self, state: &State) -> Result<(), String> {
        let result = || -> Result<(), Box<dyn std::error::Error>> {
            let temporary = self.state.with_extension("new");
            // The containing directory is root-only and calls are serialized.
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temporary)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
            file.write_all(&storage::encode(
                state,
                &self.policy,
                storage::SILKBOARD_ADAPTER,
            )?)?;
            file.sync_all()?;
            fs::rename(temporary, &self.state)?;
            File::open(self.state.parent().ok_or("missing state parent")?)?
                .sync_all()?;
            Ok(())
        };
        result().map_err(|e| format!("persist updater state: {e}"))
    }

    fn current(&mut self) -> Result<Release, String> {
        let directory =
            fs::read_link(&self.current).map_err(|e| e.to_string())?;
        self.identity(&directory)
    }

    fn refresh(&mut self) -> Result<Vec<String>, String> {
        self.fetch_git(&[
            "fetch",
            "--no-tags",
            "--prune",
            &format!("https://github.com/{}.git", self.policy.repository()),
            "+refs/tags/*:refs/tags/*",
        ])?;
        Ok(self
            .fetch_git(&[
                "for-each-ref",
                "--format=%(refname:strip=2)",
                "refs/tags/",
            ])?
            .lines()
            .map(str::to_owned)
            .collect())
    }

    fn admit(&mut self, tag: &str) -> Result<Option<Release>, String> {
        Ok(self.inspect(tag)?.release().cloned())
    }

    fn inspect(&mut self, tag: &str) -> Result<Admission, String> {
        if Version::parse(tag).is_none() {
            return Err("invalid candidate tag".into());
        }
        let reference = format!("refs/tags/{tag}");
        let object = match self.fetch_git(&["cat-file", "tag", &reference]) {
            Ok(object) => object,
            Err(error) => {
                eprintln!("Reject {tag}: {error}");
                return Ok(Admission::Rejected {
                    reason: "invalid annotated tag".into(),
                });
            }
        };
        let release = match Release::from_tag(tag, &object) {
            Ok(release) => release,
            Err(error) => {
                eprintln!("Reject {tag}: {error}");
                return Ok(Admission::Rejected { reason: error });
            }
        };
        if self
            .fetch_git(&[
                "-c",
                "gpg.ssh.allowedSignersFile=/etc/silkboard/release-signers",
                "-c",
                "gpg.ssh.program=/usr/bin/ssh-keygen",
                "verify-tag",
                &reference,
            ])
            .is_err()
        {
            eprintln!("Reject {tag}: untrusted signature");
            return Ok(Admission::Rejected {
                reason: "tag signature or target was rejected".into(),
            });
        }
        if self
            .fetch_git(&["cat-file", "-t", release.commit()])?
            .trim()
            != "commit"
        {
            return Ok(Admission::Rejected {
                reason: "tag target is not a commit".into(),
            });
        }
        self.evidence(&release)
    }

    fn build(&mut self, release: &Release) -> Result<(), String> {
        let destination = self.directory(release);
        if self.current()? == *release {
            return Err("refuse to rebuild current release".into());
        }
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(|e| e.to_string())?;
        }
        let tag = release.tag();
        let bundle = format!("/var/lib/silkboard-fetch/source-{tag}.bundle");
        self.run("rm", &["-f", "--", &bundle])?;
        self.fetch_git(&[
            "bundle",
            "create",
            &bundle,
            &format!("refs/tags/{tag}"),
        ])?;
        self.run(&format!("{TOOLS}/snapshot-bundle"), &[tag])?;
        let source = format!("/var/lib/silkboard-build/source-{tag}");
        let output = format!("/var/lib/silkboard-build/output-{tag}");
        self.run(
            "runuser",
            &["-u", "silkbuild", "--", "rm", "-rf", "--", &source, &output],
        )?;
        self.run(
            "runuser",
            &[
                "-u",
                "silkbuild",
                "--",
                "env",
                "-i",
                "HOME=/var/lib/silkboard-build",
                "PATH=/usr/bin:/bin",
                &format!("{TOOLS}/build"),
                "/var/lib/silkboard-input/source.bundle",
                tag,
                release.commit(),
            ],
        )?;
        self.run(&format!("{TOOLS}/stage"), &[tag, release.commit()])?;
        if self.identity(&destination)? != *release {
            return Err("staged identity mismatch".into());
        }
        self.run("sync", &["-f", "/opt/silkboard/releases"])?;
        Ok(())
    }

    fn activate(&mut self, release: &Release) -> Result<(), String> {
        let directory = self.directory(release);
        if self.identity(&directory)? != *release {
            return Err("activation identity mismatch".into());
        }
        let temporary = self.current.with_extension("new");
        if temporary.symlink_metadata().is_ok() {
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
        }
        symlink(directory, &temporary).map_err(|e| e.to_string())?;
        fs::rename(temporary, &self.current).map_err(|e| e.to_string())?;
        File::open(self.current.parent().ok_or("missing current parent")?)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.run("systemctl", &["reset-failed", "silkboard.service"])?;
        self.run("systemctl", &["restart", "silkboard.service"])?;
        Ok(())
    }

    fn healthy(&mut self, release: &Release) -> Result<(), String> {
        for attempt in 0..30 {
            let status = self
                .run(
                    "curl",
                    &[
                        "--fail",
                        "--silent",
                        "--show-error",
                        "--max-time",
                        "2",
                        "http://127.0.0.1:3000/api/status",
                    ],
                )
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok());
            if status.is_some_and(|s| {
                s["health"] == "ok"
                    && s["release_tag"] == release.tag()
                    && s["commit"] == release.commit()
                    && s["version"] == release.tag()[1..]
            }) && self
                .run(
                    "curl",
                    &[
                        "--fail",
                        "--silent",
                        "--show-error",
                        "--max-time",
                        "2",
                        "http://127.0.0.1:3000/health",
                    ],
                )
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .is_some_and(|s| s["status"] == "ok")
            {
                return Ok(());
            }
            if attempt < 29 {
                self.run("sleep", &["1"])?;
            }
        }
        Err(format!(
            "health or exact identity failed for {}",
            release.tag()
        ))
    }
}

fn incomplete_run(run: &Value) -> Admission {
    if matches!(
        run["status"].as_str(),
        Some("queued" | "in_progress" | "waiting" | "pending" | "requested")
    ) {
        Admission::Waiting {
            reason: "release suite is incomplete".into(),
        }
    } else {
        Admission::Rejected {
            reason: "release run failed or did not match installed policy"
                .into(),
        }
    }
}

fn classify_incomplete(response: &Value) -> Admission {
    match response["workflow_runs"].as_array() {
        Some(runs) if runs.is_empty() => Admission::Waiting {
            reason: "release suite has not run".into(),
        },
        Some(runs) => incomplete_run(
            runs.iter()
                .max_by_key(|r| r["id"].as_u64().unwrap_or(0))
                .expect("nonempty runs"),
        ),
        None => Admission::Rejected {
            reason: "invalid release run response".into(),
        },
    }
}
