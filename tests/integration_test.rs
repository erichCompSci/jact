use tokio::time::sleep;
use tokio::time::Duration as TDuration;
use std::time::Duration as SDuration;
use jact;
use jact::JobSchedulerInterface;
use jact::JobInterface;


#[tokio::test]
async fn add_one_job() -> Result<(), Box<dyn std::error::Error + 'static>>
{

    let mut sched = jact::JobScheduler::create();

    sched.add(jact::Job::new_one_shot(SDuration::from_secs(2), |_, _| Box::pin(async move {
        eprintln!("Did this work? lol"); Ok(()) }))?).await.unwrap();

    eprintln!("Past adding to sched!");

    sleep(TDuration::from_secs(4)).await;
    eprintln!("Past sleep for 4 secs!");

    Ok(())

}
