use crate::job_scheduler::JobsSchedulerLocked;
use cron::Schedule;
use chrono::Utc;
use std::str::FromStr;
use tokio::sync::RwLock;
use tokio::time::{Duration,sleep};
use std::sync::Arc;
use uuid::Uuid;
use std::pin::Pin;
use std::fmt;
use std::future::Future;
use std::error::Error;
use std::time::Instant;
use async_trait::async_trait;


pub type JobToRunAsync = dyn FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync;

///
/// A schedulable Job
pub type JobLocked = Arc<RwLock<JobType>>;

#[derive(Debug)]
pub enum JobError{
    StoppedError(String),
    CreationError(String),
    InvalidArgument(String),
    WrappedError(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            JobError::StoppedError(err_message) => write!(f, " Job stopped error -- {}", err_message),
            JobError::CreationError(err_message) => write!(f, " Job creation error -- {}", err_message),
            JobError::InvalidArgument(err_message) => write!(f, " Invalide argument error -- {}", err_message),
            JobError::WrappedError(err_message) => write!(f, " Wrapped error -- {}", err_message),
        }
    }
}

impl Error for JobError{
    fn source(&self) -> Option<&(dyn Error + 'static)> 
    { 
        match *self {
            _ => None
        }
    }
}

#[async_trait]
pub trait LockedJobInterface {

    #[doc(hidden)]
    fn make_new_cron_job<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized;


    fn new_cron_job<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized;

    #[doc(hidden)]
    fn make_one_shot_job(duration: Duration, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>>
    where 
        Self: Sized;


    fn new_one_shot<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
            Self: Sized;

    #[doc(hidden)]
    fn make_new_one_shot_at_an_instant(instant: std::time::Instant, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>>
    where 
        Self: Sized;

    fn new_one_shot_at_instant<T>(instant: std::time::Instant, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized;

    #[doc(hidden)]
    fn make_new_repeated(duration: Duration, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    fn new_repeated<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
            Self: Sized;

    async fn get_job_id(&self) -> Uuid;

}

#[async_trait]
pub trait Job {
    async fn job_id(&self) -> Uuid;
    async fn run(&mut self, jobs: JobsSchedulerLocked);
    async fn stop(&mut self) -> ();
    //fn stopped(&self) -> bool;  // Do I need this? I think not
}

pub struct JobHandle {
    pub schedule:       Option<Schedule>,
    pub run_async:      Arc<RwLock<JobToRunAsync>>,
    pub time_til_next:  Option<Duration>,
    pub job_id:         Uuid,
    pub stopped:        bool,
}

pub enum JobType {
    CronJob(JobHandle),
    OneShot(JobHandle),
    Repeated(JobHandle)
}

#[async_trait]
impl Job for JobLocked
{
    async fn job_id(&self) -> Uuid {
        let read_lock = self.read().await;
        match *read_lock
        {
            JobType::CronJob(ref handle)
          | JobType::OneShot(ref handle)
          | JobType::Repeated(ref handle) => { handle.job_id }
        }
    }

    async fn run(&mut self, jobs: JobsSchedulerLocked) {
        let job_copy = self.clone();
        //let job_scheduler_copy = jobs.clone();
        tokio::task::spawn(async move {

            loop {
                //So I can drop the read lock during the sleep
                let the_duration : Duration;
                //let uuid : Uuid;
                {
                    let read_lock = job_copy.read().await;
                    match *read_lock
                    {
                        JobType::CronJob(ref handle) => 
                        { 
                            let next_date_time = handle.schedule.as_ref().ok_or(JobError::InvalidArgument("Schedule is None".to_string()))?;
                            let next_date_time = next_date_time.upcoming(Utc).next();
                            if let Some(date_time) = next_date_time 
                            {
                                the_duration = (date_time - Utc::now()).to_std().map_err(|err| { JobError::InvalidArgument(format!("{:#?}", err.source())) })?;
                            }
                            else
                            {
                                the_duration = Duration::from_secs(0) 
                            }
                            //uuid = handle.job_id;
                        },
                        JobType::OneShot(ref handle)
                      | JobType::Repeated(ref handle) => 
                      { 
                          the_duration = handle.time_til_next.unwrap_or(Duration::from_secs(0)); 
                          //uuid = handle.job_id;
                      }
                    }
                }

                //println!("Uuid: {} sleeping for {}", uuid, the_duration.as_secs());
                sleep(the_duration).await;

                let async_func;
                let the_job_id;
                //Pick the read lock up and check that we still have work to do
                {
                    let read_lock = job_copy.read().await;
                    match *read_lock
                    {
                        JobType::CronJob(ref handle)
                      | JobType::OneShot(ref handle)
                      | JobType::Repeated(ref handle) => { 
                            if handle.stopped 
                            { 
                                return Err(JobError::StoppedError("Job was stopped manually!".to_string())); 
                            }
                            async_func = handle.run_async.clone();
                            the_job_id = handle.job_id.clone();
                      },
                    };
                }
                //Pick up the run_async lock
                {
                    //println!("Uuid: {} about to grab write lock and wait", uuid);
                    let mut write_lock = async_func.write().await;
                    let future = (*write_lock)(the_job_id, jobs.clone());
                    future.await.map_err(|err| { JobError::WrappedError(err.to_string()) })?;
                }


                //Pick up the write lock and set up the next round
                {
                    let mut write_lock = job_copy.write().await;
                    if let JobType::OneShot(ref mut handle) = *write_lock
                    {
                        handle.stopped = true;
                        return Ok(());
                    }
                }
            }

        });
    }

    async fn stop(&mut self) -> ()
    {
        let mut write_lock = self.write().await;
        match *write_lock
        {
            JobType::CronJob(ref mut handle)
          | JobType::Repeated(ref mut handle)
          | JobType::OneShot(ref mut handle) => { handle.stopped = true; }
        };
    }
}


#[async_trait]
impl LockedJobInterface for JobLocked {

    /// Create a new async cron job.
    ///
    /// ```rust,ignore
    /// let mut sched = JobScheduler::new();
    /// // Run at second 0 of the 15th minute of the 6th, 8th, and 10th hour
    /// // of any day in March and June that is a Friday of the year 2017.
    /// let job = Job::new("0 15 6,8,10 * Mar,Jun Fri 2017", |_uuid, _lock| Box::pin( async move {
    ///             println!("{:?} Hi I ran", chrono::Utc::now());
    ///         }));
    /// sched.add(job)
    /// tokio::spawn(sched.start());
    /// ```
    fn make_new_cron_job<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized,
    {
        let schedule: Schedule = Schedule::from_str(schedule)?;
        Ok(Arc::new(RwLock::new(JobType::CronJob(JobHandle {
                                                schedule: Some(schedule),
                                                run_async: Arc::new(RwLock::new(run)),
                                                time_til_next: None,
                                                job_id: Uuid::new_v4(),
                                                stopped: false }))))


    }

   /// Create a new cron job.
   ///
   /// ```rust,ignore
   /// let mut sched = JobScheduler::new();
   /// // Run at second 0 of the 15th minute of the 6th, 8th, and 10th hour
   /// // of any day in March and June that is a Friday of the year 2017.
   /// let job = Job::new_cron_job("0 15 6,8,10 * Mar,Jun Fri 2017", |_uuid, _lock| {
   ///             println!("{:?} Hi I ran", chrono::Utc::now());
   ///         });
   /// sched.add(job)
   /// tokio::spawn(sched.start());
   /// ```
    fn new_cron_job<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized,
    {
        JobLocked::make_new_cron_job(schedule, run)
    }


    fn make_one_shot_job(duration: Duration, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>>
    where 
        Self: Sized,
    {

        Ok(Arc::new(RwLock::new(JobType::OneShot(JobHandle {
                                                    schedule: None,
                                                    run_async,
                                                    time_til_next: Some(duration),
                                                    job_id: Uuid::new_v4(),
                                                    stopped: false }))))
    }

    /// Create a new async one shot job.
    ///
    /// This is checked if it is running only after 500ms in 500ms intervals.
    /// ```rust,ignore
    /// let mut sched = JobScheduler::new();
    /// let job = Job::new_one_shot(Duration::from_secs(18), |_uuid, _l| Box::pin(async move {
    ///            println!("{:?} I'm only run once", chrono::Utc::now());
    ///        }));
    /// sched.add(job)
    /// tokio::spawn(sched.start());
    /// ```
    fn new_one_shot<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
            Self: Sized,
    {
        JobLocked::make_one_shot_job(duration, Arc::new(RwLock::new(run)))
    }

    fn make_new_one_shot_at_an_instant(instant: std::time::Instant, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>>
    where 
        Self: Sized,
    {
        let first = Instant::now();
        if instant < first
        {
            return Err(Box::new(JobError::CreationError("Instant is in the past".to_string())));
        }
        let job_duration = instant - first;
        Ok(Arc::new(RwLock::new(JobType::OneShot(JobHandle {
                                                    schedule: None,
                                                    run_async,
                                                    time_til_next: Some(job_duration),
                                                    job_id: Uuid::new_v4(),
                                                    stopped: false }))))
    }

    /// Create a new async one shot job that runs at an instant
    ///
    /// ```rust,ignore
    /// // Run after 20 seconds
    /// let mut sched = JobScheduler::new();
    /// let instant = std::time::Instant::now().checked_add(std::time::Duration::from_secs(20));
    /// let job = Job::new_one_shot_at_instant(instant, |_uuid, _lock| Box::pin(async move {println!("I run once after 20 seconds")}) );
    /// sched.add(job)
    /// tokio::spawn(sched.start());
    /// ```
    fn new_one_shot_at_instant<T>(instant: std::time::Instant, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
        Self: Sized,
    {
        JobLocked::make_new_one_shot_at_an_instant(instant, Arc::new(RwLock::new(run)))
    }

    fn make_new_repeated(duration: Duration, run_async: Arc<RwLock<JobToRunAsync>>) -> Result<Self, Box<dyn std::error::Error>> 
    where 
        Self: Sized,
    {

        let job = JobHandle {
                        schedule: None,
                        run_async,
                        time_til_next: Some(duration),
                        job_id: Uuid::new_v4(),
                        stopped: false
        };

        Ok(Arc::new(RwLock::new(JobType::Repeated(job))))
    }

    /// Create a new async repeated job.
    ///
    /// This is checked if it is running only after 500ms in 500ms intervals.
    /// ```rust,ignore
    /// let mut sched = JobScheduler::new();
    /// let job = Job::new_repeated(Duration::from_secs(8), |_uuid, _lock| Box::pin(async move {
    ///     println!("{:?} I'm repeated every 8 seconds", chrono::Utc::now());
    /// }));
    /// sched.add(job)
    /// tokio::spawn(sched.start());
    /// ```
    fn new_repeated<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T:  FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + 'static>>> + Send + Sync>> + Send + Sync,
            Self: Sized,
    {
        JobLocked::make_new_repeated(duration, Arc::new(RwLock::new(run)))
    }

    async fn get_job_id(&self) -> Uuid
    {
        self.job_id().await
    }

}
