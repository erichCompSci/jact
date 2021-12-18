use tokio_test::block_on;
use tokio::time::sleep;
use tokio::time::Duration as TDuration;
use std::time::Duration as SDuration;
use jact;
use jact::JobSchedulerInterface;
use jact::JobInterface;
use jact::JobLocked;
use jact::Job;


#[tokio::test]
async fn add_one_job() -> Result<(), Box<dyn std::error::Error + 'static>>
{

    let mut sched = jact::JobScheduler::create();

    sched.add(jact::JobLocked::new_one_shot(SDuration::from_secs(2), |_, _| Box::pin(async move {
        eprintln!("Did this work? lol"); }))?).await.unwrap();

    eprintln!("Past adding to sched!");

    sleep(TDuration::from_secs(4)).await;
    eprintln!("Past sleep for 4 secs!");

    Ok(())

}
