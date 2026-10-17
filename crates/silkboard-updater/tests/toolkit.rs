//! Library consumers receive outcomes without parsing terminal output.

use silkboard_updater::{
    Backend, Release, State, Transaction,
    cli::{self, Operation, Report},
    outcome::{Admission, Error, Outcome, Stage},
    profile::Policy,
};

fn release(tag: &str) -> Release {
    Release::new(tag, &"a".repeat(40)).unwrap()
}
struct Host {
    state: State,
    current: Release,
    admission: Admission,
    calls: Vec<&'static str>,
}
impl Host {
    fn new(admission: Admission) -> Self {
        Self {
            state: State::default(),
            current: release("v0.1.1"),
            admission,
            calls: vec![],
        }
    }
}
impl Backend for Host {
    fn load(&mut self) -> Result<State, String> {
        self.calls.push("load");
        Ok(self.state.clone())
    }
    fn save(&mut self, state: &State) -> Result<(), String> {
        self.calls.push("save");
        self.state = state.clone();
        Ok(())
    }
    fn current(&mut self) -> Result<Release, String> {
        Ok(self.current.clone())
    }
    fn refresh(&mut self) -> Result<Vec<String>, String> {
        self.calls.push("fetch");
        Ok(vec!["v0.2.0".into()])
    }
    fn admit(&mut self, _: &str) -> Result<Option<Release>, String> {
        Ok(self.admission.release().cloned())
    }
    fn inspect(&mut self, _: &str) -> Result<Admission, String> {
        Ok(self.admission.clone())
    }
    fn build(&mut self, _: &Release) -> Result<(), String> {
        self.calls.push("build");
        Ok(())
    }
    fn activate(&mut self, release: &Release) -> Result<(), String> {
        self.current = release.clone();
        Ok(())
    }
    fn healthy(&mut self, _: &Release) -> Result<(), String> {
        Ok(())
    }
}
fn execute(host: &mut Host, operation: &Operation) -> Report {
    cli::execute(operation, host, &Policy::silkboard(), "fixture-v1").unwrap()
}

#[test]
fn stable_structured_reports_cover_waiting_rejection_activation_and_recovery() {
    for admission in [
        Admission::Waiting {
            reason: "checks running".into(),
        },
        Admission::Rejected {
            reason: "untrusted".into(),
        },
    ] {
        let mut host = Host::new(admission.clone());
        let Report::Polled {
            result:
                Outcome::Unchanged {
                    current,
                    not_admitted,
                },
        } = execute(&mut host, &Operation::Poll)
        else {
            panic!("unchanged expected")
        };
        assert_eq!(current.tag(), "v0.1.1");
        assert_eq!(not_admitted[0].admission, admission);
        assert!(!host.calls.contains(&"build"));
    }
    let candidate = release("v0.2.0");
    let mut host = Host::new(Admission::Eligible {
        release: candidate.clone(),
    });
    let report = execute(&mut host, &Operation::Poll);
    assert!(
        matches!(&report, Report::Polled { result: Outcome::Activated { current, .. } } if current == &candidate)
    );
    assert!(cli::human(&report).unwrap().contains("Activated v0.2.0"));
    host.state.pending = Some(Transaction {
        previous: release("v0.1.1"),
        candidate,
    });
    host.state.blocked.insert("v0.2.0".into());
    host.calls.clear();
    let report = execute(&mut host, &Operation::Poll);
    assert!(
        matches!(&report, Report::Polled { result: Outcome::Recovered { current, .. } } if current.tag() == "v0.1.1")
    );
    assert!(!host.calls.contains(&"fetch"));
    assert!(cli::human(&report).unwrap().contains("Restored v0.1.1"));
}

#[test]
fn status_inspection_and_retry_do_not_build_or_activate() {
    let mut host = Host::new(Admission::Eligible {
        release: release("v0.2.0"),
    });
    let report = execute(&mut host, &Operation::Status);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &cli::machine(&report).unwrap()
        )
        .unwrap()["schema"],
        1
    );
    assert_eq!(
        serde_json::to_value(&report).unwrap()["adapter"],
        "fixture-v1"
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(
            &cli::human(&report).unwrap()
        )
        .unwrap()
        .get("blocked")
        .is_some()
    );
    assert_eq!(host.calls, ["load"]);
    let inspected = execute(&mut host, &Operation::Inspect("v0.2.0".into()));
    assert!(cli::human(&inspected).unwrap().contains("Eligible v0.2.0"));
    host.admission = Admission::NotAdmitted;
    assert!(
        cli::human(&execute(&mut host, &Operation::Inspect("v0.2.0".into())))
            .is_err()
    );
    host.state.blocked.insert("v0.2.0".into());
    let retried = execute(&mut host, &Operation::Retry("v0.2.0".into()));
    assert!(!host.state.blocked.contains("v0.2.0"));
    assert!(
        cli::human(&retried)
            .unwrap()
            .contains("admission remains required")
    );
    assert!(!host.calls.contains(&"build"));
    assert!(
        cli::human(&execute(&mut host, &Operation::Help))
            .unwrap()
            .contains("usage:")
    );
}

#[test]
fn malformed_requests_and_wrong_admitted_tag_fail_before_build() {
    for args in [
        vec![],
        vec!["--profile"],
        vec!["--profile", "other", "status"],
        vec!["inspect", "../bad"],
        vec!["retry", "--force"],
        vec!["unknown"],
    ] {
        assert!(
            cli::parse(
                &args.into_iter().map(str::to_owned).collect::<Vec<_>>()
            )
            .is_err()
        );
    }
    for args in [
        vec!["status"],
        vec!["--json", "--profile", "silkboard", "poll"],
        vec!["inspect", "v0.2.0"],
        vec!["retry", "v0.2.0"],
        vec!["--help"],
    ] {
        assert!(
            cli::parse(
                &args.into_iter().map(str::to_owned).collect::<Vec<_>>()
            )
            .is_ok()
        );
    }
    let mut host = Host::new(Admission::Eligible {
        release: release("v0.3.0"),
    });
    let error = cli::execute(
        &Operation::Poll,
        &mut host,
        &Policy::silkboard(),
        "fixture-v1",
    )
    .unwrap_err();
    assert_eq!(error.stage, Stage::Admission);
    assert!(!host.calls.contains(&"build"));
    let mut error = Error::new(Stage::Build, "failed");
    error.restored = Some(Box::new(release("v0.1.1")));
    error.recovery = Some(Box::new(Error::new(Stage::Recovery, "unavailable")));
    assert!(
        error
            .to_string()
            .contains("restored v0.1.1; recovery failed")
    );
}
