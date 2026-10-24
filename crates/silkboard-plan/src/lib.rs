//! Bounded task-plan wire types; no transport, storage or execution authority.

mod counter;
mod validation;

pub use counter::Counter;
use serde::{Deserialize, Serialize};

/// Maximum encoded request and normalized stored snapshot size.
pub const MAX_SNAPSHOT_BYTES: usize = 262_144;
/// Maximum steps in one complete plan.
pub const MAX_STEPS: usize = 128;
/// Maximum prerequisite edges in one plan.
pub const MAX_EDGES: usize = 512;

/// Invalid protocol content; diagnostics never include input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPlan;
impl std::fmt::Display for InvalidPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid task plan")
    }
}
impl std::error::Error for InvalidPlan {}

/// Reporter-supplied native identity, not an authenticated machine identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reporter {
    /// Descriptive hostname, at most 256 ASCII graphic bytes.
    pub hostname: String,
    /// Tool label, at most 256 ASCII graphic bytes.
    pub tool: String,
    /// Native tool session identity.
    pub session_id: String,
    /// Optional native child identity.
    pub subagent_id: Option<String>,
    /// Parent metadata, permitted only with a child identity.
    pub parent_session_id: Option<String>,
    /// Optional nonblank readable label, at most 120 UTF-8 bytes.
    pub session_name: Option<String>,
}
impl Reporter {
    /// Compare immutable binding identity, excluding the readable name.
    #[must_use]
    pub fn same_identity(&self, other: &Self) -> bool {
        self.hostname == other.hostname
            && self.tool == other.tool
            && self.session_id == other.session_id
            && self.subagent_id == other.subagent_id
            && self.parent_session_id == other.parent_session_id
    }
}

/// Optional GitHub metadata. It grants no repository access or task authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueReference {
    /// Two safe ASCII repository segments, at most 100 bytes combined.
    pub repository: String,
    /// Positive issue number encoded as a canonical decimal string.
    pub number: Counter,
}

/// Agent-reported state of one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    /// Work not currently reported as started.
    Pending,
    /// Work underway with completed prerequisites.
    Active,
    /// Completion reported by the agent, not independently verified.
    Done,
    /// Work cannot continue; a reason is required.
    Blocked,
}

/// One stable step in an evolving plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Task-local stable lowercase ASCII slug, at most 64 bytes.
    pub id: String,
    /// Nonblank single-line title, at most 160 UTF-8 bytes.
    pub title: String,
    /// Optional bounded Markdown description.
    pub description: Option<String>,
    /// Unique known prerequisites, at most 32 IDs.
    pub depends_on: Vec<String>,
    /// Current reported state.
    pub state: StepState,
    /// Optional bounded report, not verification evidence.
    pub report: Option<String>,
    /// Required only for a blocked step.
    pub blocked_reason: Option<String>,
}

/// Complete current plan; counters are task-local across native sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Wire version, currently exactly 1.
    pub schema_version: u8,
    /// Canonical lowercase UUID v4, independent of session and title.
    pub task_id: String,
    /// Monotonic report order, including across handoffs.
    pub sequence: Counter,
    /// Definition revision; status-only updates can retain it.
    pub plan_revision: Counter,
    /// Reporting generation; incremented at explicit handoff.
    pub writer_generation: Counter,
    /// Native reporting identity associated with this generation.
    pub reporter: Reporter,
    /// Nonblank single-line task title.
    pub title: String,
    /// Optional bounded Markdown task description.
    pub description: Option<String>,
    /// Optional issue reference, never fetched by the dashboard.
    pub github_issue: Option<IssueReference>,
    /// Ordered presentation hints; graph edges define prerequisites.
    pub steps: Vec<Step>,
}

/// Summary computed from reported step states, not execution monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Remaining work is pending without an active or blocked step.
    Pending,
    /// At least one step is active, possibly alongside blocked branches.
    Active,
    /// Every current step is reported done.
    Completed,
    /// No step is active and at least one is blocked.
    Blocked,
}

/// Counts of reported current states, not an effort estimate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// Current number of nodes.
    pub total: usize,
    /// Nodes reported done.
    pub done: usize,
    /// Nodes reported active.
    pub active: usize,
    /// Nodes reported blocked.
    pub blocked: usize,
}

impl Snapshot {
    /// Validate the full snapshot and normalize prerequisite sets only.
    ///
    /// # Errors
    /// Rejects unsupported versions, malformed fields, invalid graphs/states
    /// and oversized normalized representations. Actual wire size is bounded
    /// separately by the transport.
    pub fn normalized(mut self) -> Result<Self, InvalidPlan> {
        validation::snapshot(&self)?;
        for step in &mut self.steps {
            step.depends_on.sort();
        }
        if serde_json::to_vec(&self).map_err(|_| InvalidPlan)?.len()
            > MAX_SNAPSHOT_BYTES
        {
            return Err(InvalidPlan);
        }
        Ok(self)
    }

    /// Compare definition fields, excluding state, notes and writer metadata.
    #[must_use]
    pub fn same_definition(&self, other: &Self) -> bool {
        self.title == other.title
            && self.description == other.description
            && self.github_issue == other.github_issue
            && self.steps.len() == other.steps.len()
            && self.steps.iter().zip(&other.steps).all(|(a, b)| {
                a.id == b.id
                    && a.title == b.title
                    && a.description == b.description
                    && a.depends_on
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        == b.depends_on.iter().collect()
            })
    }

    /// Count states in the current step set.
    #[must_use]
    pub fn counts(&self) -> Counts {
        Counts {
            total: self.steps.len(),
            done: self
                .steps
                .iter()
                .filter(|s| s.state == StepState::Done)
                .count(),
            active: self
                .steps
                .iter()
                .filter(|s| s.state == StepState::Active)
                .count(),
            blocked: self
                .steps
                .iter()
                .filter(|s| s.state == StepState::Blocked)
                .count(),
        }
    }

    /// Derive presentation state; silence and elapsed time are not inputs.
    #[must_use]
    pub fn overall_state(&self) -> TaskState {
        let c = self.counts();
        if c.total > 0 && c.done == c.total {
            TaskState::Completed
        } else if c.active > 0 {
            TaskState::Active
        } else if c.blocked > 0 {
            TaskState::Blocked
        } else {
            TaskState::Pending
        }
    }
}

/// Check canonical UUID v4 spelling without adding a runtime dependency.
#[must_use]
pub fn valid_task_id(value: &str) -> bool {
    validation::uuid(value)
}
/// Check the task-local step slug grammar.
#[must_use]
pub fn valid_step_id(value: &str) -> bool {
    validation::step_id(value)
}

/// Bounded write acknowledgement; never contains dashboard content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acknowledgement {
    /// Accepted task identity.
    pub task_id: String,
    /// Accepted task sequence.
    pub sequence: Counter,
    /// Original server receipt time in Unix milliseconds.
    pub received_at: i64,
}
