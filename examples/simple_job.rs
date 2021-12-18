use jact::{JobLocked, Job, JobScheduler, JobSchedulerInterface, JobInterface};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut sched = JobScheduler::create();

    let four_s_job_async = JobLocked::new_cron_job("1/4 * * * * *", |_uuid, _l| Box::pin(async move {
        println!("{:?} I am a cron job run async every 4 seconds", chrono::Utc::now());
    }))?;
    let four_s_job_guid = four_s_job_async.job_id().await;
    sched.add(four_s_job_async).await.unwrap();

    sched.add(
        JobLocked::new_one_shot(Duration::from_secs(16), |_uuid, _l| Box::pin( async move {
            println!("{:?} I'm only run once async", chrono::Utc::now());
        }))?).await.unwrap();

    let jja = JobLocked::new_repeated(Duration::from_secs(7), |_uuid, _l| Box::pin(async move {
        println!("{:?} I'm repeated async every 7 seconds", chrono::Utc::now());
    }))?;
    let jja_guid = jja.job_id().await;
    sched.add(jja).await.unwrap();

    let jja2 = JobLocked::new_repeated(Duration::from_secs(5), |_uuid, _l| Box::pin(async move {
        println!("{:?} I'm repeated async every 5 seconds", chrono::Utc::now());
    }))?;

    sched.add(jja2).await.unwrap();

    tokio::time::sleep(Duration::from_secs(30)).await;

    println!("{:?} Remove 4 and 7 sec jobs", chrono::Utc::now());
    sched.remove(&four_s_job_guid).await.unwrap();
    sched.remove(&jja_guid).await.unwrap();

    tokio::time::sleep(Duration::from_secs(40)).await;

    println!("{:?} Goodbye.", chrono::Utc::now());
    Ok(())
}
