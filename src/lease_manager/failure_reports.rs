use crate::lease_manager::{LeaseManager, LeasePaths, LeaseStore, RetainedFailureReport};
use crate::lease_model::LeaseFailureReport;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

impl LeaseManager {
    pub(crate) fn spawn_failure_report_janitor(&self) {
        let manager = self.clone();
        let interval_secs = self
            .config
            .failure_report_ttl_secs
            .saturating_div(10)
            .clamp(1, 60);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    manager.prune_failure_reports().await;
                }
            });
        }
    }

    pub(crate) fn failure_report_max_entries(&self) -> usize {
        self.config.failure_report_max_entries.max(1)
    }

    pub(crate) async fn prune_failure_reports(&self) {
        let mut store = self.state.lock().await;
        self.prune_failure_reports_locked(&mut store);
    }

    pub(crate) fn prune_failure_reports_locked(&self, store: &mut LeaseStore) {
        let now = now_unix_secs();
        let ttl_secs = self.config.failure_report_ttl_secs;
        store
            .failure_reports
            .retain(|_, entry| now.saturating_sub(entry.retained_at_unix_secs) < ttl_secs);

        while store.failure_reports.len() > self.failure_report_max_entries() {
            if let Some(oldest_id) = oldest_failure_report_id(store) {
                store.failure_reports.remove(&oldest_id);
            } else {
                break;
            }
        }
    }

    pub(crate) fn retain_failure_report(
        &self,
        store: &mut LeaseStore,
        id: String,
        report: LeaseFailureReport,
    ) {
        let retained_sequence = store.next_failure_report_sequence;
        store.next_failure_report_sequence = store.next_failure_report_sequence.wrapping_add(1);
        store.failure_reports.insert(
            id,
            RetainedFailureReport {
                report,
                retained_at_unix_secs: now_unix_secs(),
                retained_sequence,
            },
        );
        self.prune_failure_reports_locked(store);
    }

    pub(crate) fn paths_for(&self, id: &str) -> LeasePaths {
        let base = self.config.lease_root.join(id);
        LeasePaths {
            runtime: base.join("runtime"),
            static_assets: base.join("static"),
            state: base.join("state"),
            logs: base.join("logs"),
        }
    }
}

fn oldest_failure_report_id(store: &LeaseStore) -> Option<String> {
    store
        .failure_reports
        .iter()
        .min_by_key(|(_, entry)| entry.retained_sequence)
        .map(|(id, _)| id.clone())
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
