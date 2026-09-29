//! Scheduler: cron jobs + system notifications

use tokio_cron_scheduler::{Job, JobScheduler};
use std::sync::Arc;

pub struct Scheduler {
    sched: JobScheduler,
}

impl Scheduler {
    pub async fn new() -> Result<Self, String> {
        let sched = JobScheduler::new().await.map_err(|e| e.to_string())?;
        Ok(Self { sched })
    }
    
    pub async fn start(&self) -> Result<(), String> {
        // 每日 08:00 检查生日/周年
        let job = Job::new_async("0 0 8 * * *", |_uuid, _l| {
            Box::pin(async move { println!("Check anniversaries"); })
        }).map_err(|e| e.to_string())?;
        self.sched.add(job).await.map_err(|e| e.to_string())?;
        
        // 每日 22:00 生成每日回顾
        let job = Job::new_async("0 0 22 * * *", |_uuid, _l| {
            Box::pin(async move { println!("Generate daily recap"); })
        }).map_err(|e| e.to_string())?;
        self.sched.add(job).await.map_err(|e| e.to_string())?;
        
        // 每周日 10:00 自动备份
        let job = Job::new_async("0 0 10 * * 0", |_uuid, _l| {
            Box::pin(async move { println!("Auto backup"); })
        }).map_err(|e| e.to_string())?;
        self.sched.add(job).await.map_err(|e| e.to_string())?;
        
        self.sched.start().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}