use jact::{JobLocked, Job, JobScheduler, JobSchedulerInterface, JobInterface};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut sched = JobScheduler::create();

    println!("Creating new cron job!");
    let four_s_job_async = JobLocked::new_cron_job("1/4 * * * * *", |_uuid, _l| Box::pin(async move {
        println!("{:?} I run async every 4 seconds", chrono::Utc::now());
    }))?;
    let four_s_job_guid = four_s_job_async.job_id().await;
    sched.add(four_s_job_async).await.unwrap();
    println!("Added new job!");

    println!("Creating and adding new one shot job!");
    sched.add(
        JobLocked::new_one_shot(Duration::from_secs(16), |_uuid, _l| Box::pin( async move {
            println!("{:?} I'm only run once async", chrono::Utc::now());
        }))?).await.unwrap();

    //let jj = Job::new_repeated(Duration::from_secs(8), |_uuid, _l| {
    //    println!("{:?} I'm repeated every 8 seconds", chrono::Utc::now());
    //}).unwrap();
    //let jj_guid = jj.guid();
    //sched.add(jj);

    println!("Creating new repeated job!");
    let jja = JobLocked::new_repeated(Duration::from_secs(7), |_uuid, _l| Box::pin(async move {
        println!("{:?} I'm repeated async every 7 seconds", chrono::Utc::now());
    }))?;
    let jja_guid = jja.job_id().await;
    sched.add(jja).await.unwrap();
    println!("Added new repeated job!");

    println!("Waiting!");
    //tokio::spawn(sched.start());
    tokio::time::sleep(Duration::from_secs(30)).await;

    println!("{:?} Remove 4, 5, 7 and 8 sec jobs", chrono::Utc::now());
    //sched.remove(&five_s_job_guid);
    sched.remove(&four_s_job_guid).await.unwrap();
    //sched.remove(&jj_guid);
    sched.remove(&jja_guid).await.unwrap();

    tokio::time::sleep(Duration::from_secs(40)).await;

    println!("{:?} Goodbye.", chrono::Utc::now());
    Ok(())
}
