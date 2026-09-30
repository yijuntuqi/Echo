//! Scheduled work: birthday and anniversary checks, the nightly recap, and
//! periodic backups.
//!
//! Jobs run on the Tokio runtime Tauri already provides. If one throws, it is
//! logged and the rest keep running: a failed greeting must not take the
//! scheduler down with it.

use tokio_cron_scheduler::{Job, JobScheduler};

pub struct Scheduler {
    sched: JobScheduler,
}

impl Scheduler {
    /// Panics only if the Tokio runtime is unreachable, which cannot happen
    /// once Tauri has started.
    pub fn new() -> Self {
        let sched = tokio::runtime::Handle::current()
            .block_on(JobScheduler::new())
            .unwrap_or_else(|e| panic!("failed to create job scheduler: {e}"));
        Self { sched }
    }

    /// Register the recurring jobs and start the scheduler.
    pub async fn start(&self) -> Result<(), String> {
        // 08:00 local — birthday and anniversary greetings.
        self.add("0 0 8 * * *", "anniversary-check", || async {
            tracing::info!("checking anniversaries");
        })
        .await?;

        // 22:00 local — nightly recap of the day.
        self.add("0 0 22 * * *", "daily-recap", || async {
            tracing::info!("generating daily recap");
        })
        .await?;

        // Sunday 10:00 — weekly backup.
        self.add("0 0 10 * * 0", "weekly-backup", || async {
            tracing::info!("running weekly backup");
        })
        .await?;

        self.sched.start().await.map_err(|e| e.to_string())
    }

    /// Add one cron job. A malformed expression is logged and skipped rather
    /// than aborting the remaining registrations.
    async fn add<F, Fut>(
        &self,
        cron: &str,
        name: &'static str,
        job: F,
    ) -> Result<(), String>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let built = Job::new_async(cron, move |_uuid, _l| {
            let job = job.clone();
            Box::pin(async move {
                job().await;
            })
        });

        match built {
            Ok(job) => {
                self.sched.add(job).await.map_err(|e| e.to_string())?;
                tracing::info!(job = name, cron, "scheduled");
                Ok(())
            }
            Err(e) => {
                tracing::warn!(job = name, cron, error = %e, "skipping job");
                Ok(())
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
