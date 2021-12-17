use crate::job_scheduler::JobsSchedulerLocked;
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::str::FromStr;
use tokio::sync::RwLock;
use tokio::time::{Duration,sleep};
use std::sync::Arc;
use uuid::Uuid;
use tokio::task::JoinHandle;
use std::pin::Pin;
use std::future::Future;

pub type JobToRunAsync = dyn FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync;

///
/// A schedulable Job
pub type JobLocked = Arc<RwLock<JobType>>;

#[derive(Debug)]
pub enum JobError{
    StoppedError(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            JobError::StoppedError(err_message) => write!(f, " Job stopped error -- {}", err_message),
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

pub trait Job {
    fn job_id(&self) -> Uuid;
    fn run(&mut self, jobs: JobsSchedulerLocked);
    fn stop(&mut self) -> ();
    //fn stopped(&self) -> bool;  // Do I need this? I think not
}

struct JobHandle {
    pub schedule:       Option<Schedule>,
    pub run_async:      Box<JobToRunAsync>,
    pub time_til_next:  Option<Duration>,
    pub job_id:         Uuid,
    pub stopped:        bool,
}

pub enum JobType {
    CronJob(JobHandle),
    OneShot(JobHandle),
    Repeated(JobHandle)
}

impl Job for JobLocked
{
    fn job_id(&self) -> Uuid {
        let read_lock = self.read().await;
        self.0.job_id
    }

    fn run(&mut self, jobs: JobsSchedulerLocked) {
        let job_copy = self.clone();
        //let job_scheduler_copy = jobs.clone();
        let read_lock = self.read().await;
        tokio::task::spawn(async move {

            loop {
                //So I can drop the read lock during the sleep
                let the_type;
                let the_duration;
                {
                    let read_lock = job_copy.read().await;
                    the_type = *read_lock;
                    the_duration = read_lock.0.time_til_next.ok_or(Duration::from_secs(0));
                }
                //So I can sleep without the read lock
                match the_type {
                    JobType::CronJob => {

                    }
                    JobType::Repeated | JobType::OneShot => {
                        sleep(the_duration);
                    }
                };
                //Pick the read lock up and check that we still have work to do
                {
                    let read_lock = job_copy.read().await;
                    if read_lock.0.stopped
                    {
                        return Err(JobError::StoppedError("Job was stopped manually!".to_string()));
                    }

                    let future = (read_lock.0.run_async)(read_lock.0.job_id, jobs);
                    //eprintln!("Spawning the task");
                    //eprintln!("Running the future");
                    future.await;
                }

                //Pick up the write lock and set up the next round
                {
                    let write_lock = job_copy.write().await;
                    match job_type
                    {
                        JobType::CronJob => {

                        }
                        JobType::Repeated => {} //Do nothing, the job will repeat naturally
                        _ => { return Ok(()); }
                    }
                }
            }

        });
    }

    fn stop(&mut self) -> ()
    {
        let write_lock = self.write().await;
        self.0.stopped = true;
    }
}



//impl JobHandle {
//    fn job_type(&self) -> &JobType {
//        &JobType::CronJob
//    }
//
//    fn ran(&self) -> bool {
//        self.ran
//    }
//
//    fn set_ran(&mut self, ran: bool) {
//        self.ran = ran;
//    }
//
//    fn set_join_handle(&mut self, _handle: Option<JoinHandle<()>>) {}
//
//    fn abort_join_handle(&mut self) {}
//
//    fn stop(&self) -> bool {
//        self.stopped
//    }
//
//    fn set_stopped(&mut self) {
//        self.stopped = true;
//    }
//}

//struct NonCronJob {
//    pub run_async: Box<JobToRunAsync>,
//    pub last_tick: Option<DateTime<Utc>>,
//    pub job_id: Uuid,
//    pub ran: bool,
//    pub count: u32,
//    pub job_type: JobType,
//    pub stopped: bool,
//}
//
//impl Job for NonCronJob {
//    fn is_cron_job(&self) -> bool {
//        false
//    }
//
//    fn schedule(&self) -> Option<&Schedule> {
//        None
//    }
//
//    fn last_tick(&self) -> Option<&DateTime<Utc>> {
//        self.last_tick.as_ref()
//    }
//
//    fn set_last_tick(&mut self, tick: Option<DateTime<Utc>>) {
//        self.last_tick = tick;
//    }
//
//    fn set_count(&mut self, count: u32) {
//
//        self.count = count;
//    }
//
//    fn count(&self) -> u32 {
//        self.count
//    }
//
//    fn increment_count(&mut self) {
//        self.count = if self.count + 1 < u32::MAX {
//            self.count + 1
//        } else {
//            0
//        }; // Overflow check
//    }
//
//    fn job_id(&self) -> Uuid {
//        self.job_id
//    }
//
//    fn run(&mut self, jobs: JobsSchedulerLocked) {
//        let future = (self.run_async)(self.job_id, jobs);
//        eprintln!("Second");
//        eprintln!("Spawning the task");
//        tokio::task::spawn(async move {
//            eprintln!("Running the future");
//            future.await;
//        });
//    }
//
//    fn job_type(&self) -> &JobType {
//        &self.job_type
//    }
//
//    fn ran(&self) -> bool {
//        self.ran
//    }
//
//    fn set_ran(&mut self, ran: bool) {
//        self.ran = ran;
//    }
//
//    fn set_join_handle(&mut self, handle: Option<JoinHandle<()>>) {
//        self.join_handle = handle;
//    }
//
//    fn abort_join_handle(&mut self) {
//        let mut s: Option<JoinHandle<()>> = None;
//        std::mem::swap(&mut self.join_handle, &mut s);
//        if let Some(jh) = s {
//            self.set_join_handle(None);
//            jh.abort();
//            drop(jh);
//        }
//    }
//
//    fn stop(&self) -> bool {
//        self.stopped
//    }
//
//    fn set_stopped(&mut self) {
//        self.stopped = true;
//    }
//}

impl JobLocked {

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
    pub fn new<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync,
    {
        let schedule: Schedule = Schedule::from_str(schedule)?;
        Arc::new(RwLock::new(JobType::CronJob(JobHandle {
                                                schedule: Some(schedule),
                                                run_async: Box::new(run),
                                                time_til_next: None,
                                                job_id: Uuid::new_v4(),
                                                stopped: false })))


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
    pub fn new_cron_job<T>(schedule: &str, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync,
    {
        JobLocked::new(schedule, run)
    }


    fn make_one_shot_job(duration: Duration, run_async: Box<JobToRunAsync>) -> Result<Self, Box<dyn std::error::Error>> {
        let job = JobHandle {
            schedule: None,
            run_async,
            time_til_next: Some(duration),
            job_id: Uuid::new_v4(),
            stopped: false
        };

        Ok(Arc::new(RwLock::new(job)))
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
    pub fn new_one_shot<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync,
    {
        JobLocked::make_one_shot_job(duration, Box::new(run))
    }

    fn make_new_one_shot_at_an_instant(instant: std::time::Instant, run_async: Box<JobToRunAsync>) -> Result<Self, Box<dyn std::error::Error>> {
        let job
        let job = NonCronJob {
            run_async,
            last_tick: None,
            job_id: Uuid::new_v4(),
            join_handle: None,
            ran: false,
            count: 0,
            job_type: JobType::OneShot,
            stopped: false,
        };

        let job: Arc<RwLock<Box<dyn Job + Send + Sync + 'static>>> = Arc::new(RwLock::new(Box::new(job)));
        let job_for_trigger = job.clone();

        let jh = tokio::spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from(instant)).await;
            {
                let j = job_for_trigger.read().await;
                if j.stop() {
                    return;
                }
            }
            {
                let mut j = job_for_trigger.write().await;
                j.set_last_tick(Some(Utc::now()));
            }
        });

        {
            let mut j = job.write().await;
            j.set_join_handle(Some(jh));
        }

        Ok(Self {
            0: job
        })
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
    pub async fn new_one_shot_at_instant<T>(instant: std::time::Instant, run: T) -> Result<Self, Box<dyn std::error::Error>>
    where
        T: 'static,
        T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync,
    {
        JobLocked::make_new_one_shot_at_an_instant(instant, Box::new(run)).await
    }

    async fn make_new_repeated(duration: Duration, run_async: Box<JobToRunAsync>) -> Result<Self, Box<dyn std::error::Error>> {
        let job = NonCronJob {
            run_async,
            last_tick: None,
            job_id: Uuid::new_v4(),
            join_handle: None,
            ran: false,
            count: 0,
            job_type: JobType::Repeated,
            stopped: false,
        };

        let job: Arc<RwLock<Box<dyn Job + Send + Sync + 'static>>> = Arc::new(RwLock::new(Box::new(job)));
        let job_for_trigger = job.clone();

        let jh = tokio::spawn(async move {
            let mut interval = tokio::time::interval(duration);
            loop {
                interval.tick().await;
                {
                    let j = job_for_trigger.read().await;
                    if j.stop() {
                        return;
                    }
                }
                {
                    let mut j = job_for_trigger.write().await;
                    j.set_last_tick(Some(Utc::now()));
                }
            }
        });

        {
            let mut j = job.write().await;
            j.set_join_handle(Some(jh));
        }

        Ok(Self {
            0: job
        })
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
pub async fn new_repeated<T>(duration: Duration, run: T) -> Result<Self, Box<dyn std::error::Error>>
        where
            T: 'static,
            T: FnMut(Uuid, JobsSchedulerLocked) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync,
    {
        JobLocked::make_new_repeated(duration, Box::new(run)).await
    }

    ///
    /// The `tick` method returns a true if there was an invocation needed after it was last called
    /// This method will also change the last tick on itself
    pub async fn tick(&mut self) -> bool {
        let now = Utc::now();
        {
            let mut s = self.0.write().await;
            //*s.map(|mut s| {
            match *s.job_type() {
                JobType::CronJob => {
                    if s.last_tick().is_none() {
                        s.set_last_tick(Some(now));
                        return false;
                    }
                    let last_tick = *s.last_tick().unwrap();
                    s.set_last_tick(Some(now));
                    s.increment_count();
                    let must_run = s.schedule().unwrap()
                        .after(&last_tick)
                        .take(1)
                        .map(|na| {
                            let now_to_next = now.cmp(&na);
                            let last_to_next = last_tick.cmp(&na);

                            matches!(now_to_next, std::cmp::Ordering::Greater) &&
                                matches!(last_to_next, std::cmp::Ordering::Less)
                        })
                        .into_iter()
                        .find(|_| true)
                        .unwrap_or(false);

                    if !s.ran() && must_run {
                        s.set_ran(true);
                    }
                    must_run
                },
                JobType::OneShot => {
                    eprintln!("s: {:#?}", s.last_tick());
                    if s.last_tick().is_some() {
                        s.set_last_tick(None);
                        s.set_ran(true);
                        true
                    } else {
                        false
                    }
                },
                JobType::Repeated => {
                    if s.last_tick().is_some() {
                        s.set_last_tick(None);
                        s.set_ran(true);
                        true
                    } else {
                        false
                    }
                }
            }
        }
    }

    ///
    /// Get the GUID for the job
    ///
    pub async fn guid(&self) -> Uuid {
        let r = self.0.read().await;
        r.job_id()
    }
}
