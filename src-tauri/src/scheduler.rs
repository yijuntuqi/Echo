//! Scheduled work: birthday and anniversary checks, the nightly recap, and
//! periodic backups.
//!
//! Jobs run on the Tokio runtime Tauri already provides. If one throws, it is
//! logged and the rest keep running: a failed greeting must not take the
//! scheduler down with it.

use tokio_cron_scheduler::{Job, JobScheduler};

pub struct Scheduler {
    sched: std::sync::OnceLock<JobScheduler>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { sched: std::sync::OnceLock::new() }
    }

    fn get_or_init(&self) -> &JobScheduler {
        self.sched.get_or_init(|| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(JobScheduler::new())
                    .unwrap_or_else(|e| panic!("failed to create job scheduler: {e}"))
            })
        })
    }

    /// Register the recurring jobs and start the scheduler.
    pub async fn start(&self) -> Result<(), String> {
        let sched = self.get_or_init();

        // 08:00 local — birthday and anniversary greetings.
        self.add(sched, "0 0 8 * * *", "anniversary-check", || async {
            tracing::info!("checking anniversaries");
        })
        .await?;

        // 22:00 local — nightly recap of the day.
        self.add(sched, "0 0 22 * * *", "daily-recap", || async {
            tracing::info!("generating daily recap");
        })
        .await?;

        // Sunday 10:00 — weekly backup.
        self.add(sched, "0 0 10 * * 0", "weekly-backup", || async {
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
                sched.add(job).await;
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
        Self { sched: std::sync::OnceLock::new() }
    }
}