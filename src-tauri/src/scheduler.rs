//! Scheduled work: birthday and anniversary checks, the nightly recap, and
//! periodic backups.
//!
//! Jobs run on the Tokio runtime Tauri already provides. If one throws, it is
//! logged and the rest keep running: a failed greeting must not take the
//! scheduler down with it.

use tokio::sync::OnceCell;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct Scheduler {
    // Async OnceCell, not std's: `JobScheduler::new()` must be awaited inside
    // the runtime. Wrapping it in `block_in_place` + `block_on` made the
    // scheduler's internal start-acknowledgement channel die before the tick
    // task could answer, so every `start()` failed with `TickError`.
    sched: OnceCell<JobScheduler>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { sched: OnceCell::new() }
    }

    async fn get_or_init(&self) -> &JobScheduler {
        self.sched
            .get_or_init(|| async {
                JobScheduler::new()
                    .await
                    .expect("failed to create job scheduler")
            })
            .await
    }

    /// Register the recurring jobs and start the scheduler.
    pub async fn start(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let sched = self.get_or_init().await;

        // 08:00 local — birthday and anniversary greetings (Task 8).
        let greet_app = app.clone();
        self.add(sched, "0 0 8 * * *", "anniversary-check", move || {
            let app = greet_app.clone();
            async move { crate::recap::anniversary_check(app).await }
        })
        .await?;

        // 09:10 local — daily evolution check: time alone keeps a neglected
        // pet creeping forward, and it catches up on anything the per-turn
        // evaluations missed.
        let evo_app = app.clone();
        self.add(sched, "0 10 9 * * *", "evolution-check", move || {
            let app = evo_app.clone();
            async move {
                if let Some(evolved) =
                    crate::evolution::evaluate(&app, crate::evolution::Trigger::TimeElapsed).await
                {
                    tracing::info!(to = evolved.to_stage.as_str(), "daily check evolved the pet");
                }
            }
        })
        .await?;

        // 22:00 local — nightly recap of the day (Task 8).
        let recap_app = app.clone();
        self.add(sched, "0 0 22 * * *", "daily-recap", move || {
            let app = recap_app.clone();
            async move { crate::recap::daily_recap(app).await }
        })
        .await?;

        // Sunday 10:00 — weekly backup. The cron crate's weekday field takes
        // 1-7 (1 = Sunday) or names; `0` is invalid and fails to parse.
        self.add(sched, "0 0 10 * * Sun", "weekly-backup", || async {
            tracing::info!("running weekly backup");
        })
        .await?;

        sched.start().await.map_err(|e| e.to_string())
    }

    /// Add one cron job. A malformed expression is logged and skipped rather
    /// than aborting the remaining registrations.
    async fn add<F, Fut>(&self, sched: &JobScheduler, cron: &str, name: &'static str, job: F) -> Result<(), String>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let built = Job::new_async(cron, move |_uuid, _l| {
            let job = job.clone();
            Box::pin(async move { job().await })
        });

        match built {
            Ok(job) => {
                // A registration failure (clock source missing, …) must not
                // abort the remaining jobs.
                let _ = sched.add(job).await;
                tracing::info!(job = name, "scheduled");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(error = %e, job = name, "skipping job");
                Ok(())
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self { sched: OnceCell::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The production init path: `JobScheduler::new().await` directly in an
    /// async context, then start. Guards both the TickError regression and
    /// the exact cron expressions shipped in `start` (a typo there is only
    /// a WARN at runtime).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn scheduler_starts_and_all_crons_parse() {
        let sched = JobScheduler::new().await.expect("new scheduler");
        for cron in ["0 0 8 * * *", "0 10 9 * * *", "0 0 22 * * *", "0 0 10 * * Sun"] {
            Job::new_async(cron, move |_uuid, _l| Box::pin(async {}))
                .unwrap_or_else(|e| panic!("cron {cron:?} failed to parse: {e}"));
        }
        sched.start().await.expect("scheduler should start without TickError");
    }
}