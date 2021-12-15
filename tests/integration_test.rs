use tokio_test::block_on;
use tokio::time::sleep;
use tokio::time::Duration as TDuration;
use std::time::Duration as SDuration;
use jact;

#[tokio::test]
async fn add_one_job() -> Result<(), Box<dyn std::error::Error + 'static>>
{

    let mut sched = jact::JobScheduler::new();

    sched.add(jact::Job::new_one_shot(SDuration::from_secs(2), |_, _| Box::pin(async move {
        eprintln!("Did this work? lol"); })).await?).await.unwrap();

    eprintln!("Past adding to sched!");
    sched.tick().await.unwrap();
    eprintln!("Past first tick!");

    sleep(TDuration::from_secs(3)).await;
    eprintln!("Past sleep for 3 secs!");

    sched.tick().await.unwrap();
    sleep(TDuration::from_secs(1)).await;
    eprintln!("Past sleep for 1 secs!");
    sched.tick().await.unwrap();
    eprintln!("Past second tick!");

    Ok(())

}
