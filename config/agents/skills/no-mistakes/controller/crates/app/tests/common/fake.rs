use super::sha;
use domain::{
    ids::TaskId,
    ports::{
        Capabilities, Clock, GitHub, Harness, Isolation, LaunchRequest, LaunchResult, MergeMethod,
        PortError, StructuredOutput, Vcs,
    },
    risk::HarnessKind,
};
use std::path::Path;

pub struct Fake {
    pub reserve: bool,
    pub slot: domain::ports::SlotState,
    /// The PR `create`/`ready`/`merge`/`observe` agree on: each mutating
    /// call updates it, `observe` reads it back, so a test sees the same
    /// sequence of states a real GitHub adapter would produce.
    pub observation: std::cell::RefCell<Option<domain::delivery::PullRequest>>,
    pub clock: std::cell::Cell<u64>,
    /// The head `Vcs::head` reports for any revision. Tests move it (e.g.
    /// `fake.head.set('b')`) to simulate a new commit between snapshots.
    pub head: std::cell::Cell<char>,
    /// The harness output returned for a reviewer (or escalation) launch.
    /// Defaults to a non-JSON string so a test that never sets it still
    /// exercises the malformed-review path; a golden-path test replaces it
    /// with a valid `Review` JSON built from the real snapshot and launch.
    pub reviewer_output: std::cell::RefCell<String>,
    /// When set, every PR `set_pr` records (`create`/`ready`/`merge`) carries
    /// one required, pending check, so `observe` reports CI still running
    /// until a test clears it.
    pub pending_checks: std::cell::Cell<bool>,
    /// What `Vcs::on_main` reports for any commit. Defaults to `true`; a
    /// test flips it to simulate a historical merge or replacement PR that
    /// has since left `origin/main`.
    pub on_main: std::cell::Cell<bool>,
    /// Every worktree-mutating `Vcs` call actually made (`bind_slot`,
    /// `reuse_slot`), in order, so a rejection test can assert the
    /// forbidden effect never happened.
    pub vcs_calls: std::cell::RefCell<Vec<&'static str>>,
    /// The `session` field of the most recent `LaunchRequest` the harness
    /// received (`Some(None)` for a fresh launch, `Some(Some(id))` for a
    /// resumed one), so a test can assert whether a launch resumed a prior
    /// session.
    pub last_session: std::cell::RefCell<Option<Option<String>>>,
    /// When set, the next `GitHub::create` call fails once (simulating a
    /// network failure after the coordinator's own operation was already
    /// recorded), then clears itself so a later `create` succeeds
    /// normally.
    pub create_fails_once: std::cell::Cell<bool>,
}

impl Clock for Fake {
    fn now(&self) -> u64 {
        self.clock.get()
    }

    fn sleep(&self, seconds: u64) {
        self.clock.set(self.clock.get().saturating_add(seconds));
    }
}

impl Vcs for Fake {
    fn head(&self, _revision: &str) -> Result<domain::ids::Sha, PortError> {
        sha(self.head.get())
    }
    fn on_main(&self, _commit: &domain::ids::Sha) -> Result<bool, PortError> {
        Ok(self.on_main.get())
    }
    fn changed_paths(
        &self,
        _base: &domain::ids::Sha,
        _head: &domain::ids::Sha,
    ) -> Result<Vec<String>, PortError> {
        Ok(Vec::new())
    }
    fn changed_lines(
        &self,
        _base: &domain::ids::Sha,
        _head: &domain::ids::Sha,
    ) -> Result<u32, PortError> {
        Ok(0)
    }
    fn commit_paths(
        &self,
        _base: &domain::ids::Sha,
        _head: &domain::ids::Sha,
    ) -> Result<Vec<Vec<String>>, PortError> {
        Ok(Vec::new())
    }
    fn ignored(&self, _path: &Path) -> Result<bool, PortError> {
        Ok(true)
    }
    fn reserve(&self, _task: &TaskId) -> Result<bool, PortError> {
        Ok(self.reserve)
    }
    fn inspect_slot(
        &self,
        _slot: &domain::ids::SlotId,
    ) -> Result<domain::ports::SlotState, PortError> {
        Ok(self.slot.clone())
    }
    fn bind_slot(
        &self,
        _task: &TaskId,
        _slot: &domain::ids::SlotId,
        _branch: &str,
    ) -> Result<(), PortError> {
        self.vcs_calls.borrow_mut().push("bind_slot");
        Ok(())
    }
    fn reuse_slot(&self, _slot: &domain::ids::SlotId, _branch: &str) -> Result<(), PortError> {
        self.vcs_calls.borrow_mut().push("reuse_slot");
        Ok(())
    }
    fn clean_slot(&self, _slot: &domain::ids::SlotId, _delete: bool) -> Result<(), PortError> {
        Ok(())
    }
}

impl GitHub for Fake {
    fn create(
        &self,
        _title: &str,
        _body: &Path,
        head: &domain::ids::Sha,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        if self.create_fails_once.replace(false) {
            return Err(PortError("simulated create failure".into()));
        }
        self.set_pr(
            domain::ids::PrNumber(1),
            domain::delivery::PrState::Open,
            true,
            head,
            None,
        )
    }
    fn ready(
        &self,
        number: domain::ids::PrNumber,
        head: &domain::ids::Sha,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        self.set_pr(number, domain::delivery::PrState::Open, false, head, None)
    }
    fn merge(
        &self,
        number: domain::ids::PrNumber,
        head: &domain::ids::Sha,
        _method: MergeMethod,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        self.set_pr(
            number,
            domain::delivery::PrState::Merged,
            false,
            head,
            Some(head.clone()),
        )
    }
    fn observe(
        &self,
        _number: domain::ids::PrNumber,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        let mut pr = self
            .observation
            .borrow()
            .clone()
            .ok_or_else(|| PortError("unused".into()))?;
        pr.checks = self.checks();
        Ok(pr)
    }
}

impl Fake {
    /// The required check `create`/`ready`/`merge` and `observe` all
    /// report, read fresh from `pending_checks` on every call so a test can
    /// flip it between an `observe`/`poll-checks` and see the change,
    /// exactly as a real GitHub adapter's next read would.
    fn checks(&self) -> Vec<domain::delivery::Check> {
        vec![domain::delivery::Check {
            name: "ci".into(),
            state: if self.pending_checks.get() {
                domain::delivery::CheckState::Pending
            } else {
                domain::delivery::CheckState::Pass
            },
            required: true,
        }]
    }

    /// Records the PR state a `create`/`ready`/`merge` call produces so a
    /// later `observe` (via `observe-pr` or `poll-checks`) reads it back,
    /// the way a real GitHub adapter's own state would be visible on the
    /// next read.
    fn set_pr(
        &self,
        number: domain::ids::PrNumber,
        state: domain::delivery::PrState,
        draft: bool,
        head: &domain::ids::Sha,
        merge: Option<domain::ids::Sha>,
    ) -> Result<domain::delivery::PullRequest, PortError> {
        let checks = self.checks();
        let pr = domain::delivery::PullRequest {
            number,
            state,
            draft,
            head: head.clone(),
            merge,
            checks,
        };
        *self.observation.borrow_mut() = Some(pr.clone());
        Ok(pr)
    }
}

impl Harness for Fake {
    fn kind(&self) -> HarnessKind {
        HarnessKind::Pi
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            structured_output: StructuredOutput::SelfValidated,
            isolation: Isolation::ToolRestriction,
        }
    }
    fn run(
        &self,
        request: &LaunchRequest,
        started: &mut dyn FnMut(domain::ports::ProcessIdentity) -> Result<(), PortError>,
    ) -> Result<LaunchResult, PortError> {
        *self.last_session.borrow_mut() = Some(request.session.clone());
        started(domain::ports::ProcessIdentity {
            pid: 1,
            start_ticks: 2,
            group: 1,
        })?;
        let (session, output) = if request.reviewer {
            (
                "reviewer-session".to_owned(),
                self.reviewer_output.borrow().clone(),
            )
        } else {
            ("implementer-session".to_owned(), "done".to_owned())
        };
        Ok(LaunchResult {
            session,
            output,
            tokens: None,
        })
    }
}
