use jact::{Job, JobScheduler, JobSchedulerInterface, JobInterface, JobError};
use std::time::Duration;

async fn testing_theory(mut thing: JobScheduler) -> Result<(), JobError>
{
    thing.add(Job::new_one_shot(Duration::from_secs(2), |_uuid, _l| Box::pin(async move { println!("Extra"); Ok(()) }))?).await.unwrap();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut sched = JobScheduler::create();

    let four_s_job_async = Job::new_cron_job("1/4 * * * * *", |_uuid, l| Box::pin(async move {
        println!("{:?} I am a cron job run async every 4 seconds", chrono::Utc::now()); 
        let job_catch = l.clone();
        testing_theory(job_catch).await?;
        Ok(())
    }))?;
    let four_s_job_guid = four_s_job_async.get_job_id().await;
    sched.add(four_s_job_async).await.unwrap();

    sched.add(
        Job::new_one_shot(Duration::from_secs(16), |_uuid, _l| Box::pin( async move {
            println!("{:?} I'm only run once async", chrono::Utc::now()); Ok(())
        }))?).await.unwrap();

    let jja = Job::new_repeated(Duration::from_secs(7), |_uuid, _l| Box::pin(async move {
        println!("{:?} I'm repeated async every 7 seconds", chrono::Utc::now()); Ok(())
    }))?;
    let jja_guid = jja.get_job_id().await;
    sched.add(jja).await.unwrap();

    let jja2 = Job::new_repeated(Duration::from_secs(5), |_uuid, _l| Box::pin(async move {
        println!("{:?} I'm repeated async every 5 seconds", chrono::Utc::now()); Ok(())
    }))?;

    sched.add(jja2).await.unwrap();

    tokio::time::sleep(Duration::from_secs(30)).await;

    println!("{:?} Remove 4 and 7 sec jobs", chrono::Utc::now());
    sched.remove(&four_s_job_guid).await.unwrap();
    sched.remove(&jja_guid).await.unwrap();

    tokio::time::sleep(Duration::from_secs(40)).await;

    println!("Cleaning...");
    sched.clean().await.unwrap();
    println!("{:?} Goodbye.", chrono::Utc::now());
    Ok(())
}
