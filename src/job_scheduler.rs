use crate::job::{JobLocked, JobType, Job, JobError};
use tokio::sync::RwLock;
use std::sync::Arc;
use uuid::Uuid;
use std::fmt;
use std::error::Error;
use std::collections::HashMap;
use async_trait::async_trait;
use std::ops::DerefMut;

// The JobScheduler contains and executes the scheduled jobs.
//pub struct JobsSchedulerLocked(Arc<RwLock<JobScheduler>>);
pub type JobsSchedulerLocked = Arc<RwLock<JobScheduler>>;


//impl Clone for JobsSchedulerLocked {
//    fn clone(&self) -> Self {
//        JobsSchedulerLocked(self.0.clone())
//    }
//}

#[derive(Default,Debug)]
pub struct JobScheduler {
    jobs_to_run: HashMap<Uuid, JobLocked>,
}

#[derive(Debug)]
pub enum JobSchedulerError{
    InvalidArgument(String),
}

impl fmt::Display for JobSchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self {
            JobSchedulerError::InvalidArgument(err_message) => write!(f, " Invalid Argument error -- {}", err_message),
        }
    }
}

impl Error for JobSchedulerError{
    fn source(&self) -> Option<&(dyn Error + 'static)> 
    { 
        match *self {
            _ => None
        }
    }
}


async fn enumerate_removal(job: &mut JobLocked, 
                           retained: &mut HashMap<Uuid, JobLocked>)
{
    let read_lock = job.read().await;
    match *read_lock
    {
        JobType::CronJob(ref handle) 
      | JobType::Repeated(ref handle)
      | JobType::OneShot(ref handle) => {
          if handle.stopped
          {
              retained.insert(handle.job_id, job.clone());
          }
      }
    }
}

#[async_trait]
pub trait LockedSchedInterface {
    fn create() -> JobsSchedulerLocked;
    async fn add(&mut self, job: JobLocked) -> Result<(), JobError>;
    async fn remove(&mut self, to_be_removed: &Uuid) -> Result<(), JobError>;
    async fn clean(&mut self) -> Result<(), JobError>;
}

#[async_trait]
impl LockedSchedInterface for JobsSchedulerLocked {

    /// Create a new `JobSchedulerLocked`.
    fn create() -> JobsSchedulerLocked {
        let r = JobScheduler {
            jobs_to_run : HashMap::new(),
        };
        Arc::new(RwLock::new(r))
    }

    /// Add a job to the `JobScheduler`
    ///
    /// ```rust,ignore
    /// use tokio_cron_scheduler::{Job, JobScheduler, JobToRun};
    /// let mut sched = JobScheduler::new();
    /// sched.add(Job::new("1/10 * * * * *".parse().unwrap(), || {
    ///     println!("I get executed every 10 seconds!");
    /// }));
    /// ```
    async fn add(&mut self, job: JobLocked) -> Result<(), JobError> {
        {
            let mut self_w = self.write().await;
            let uuid;
            {
                let job_read = job.read().await;
                match *job_read
                {
                    JobType::CronJob(ref handle)
                  | JobType::Repeated(ref handle)
                  | JobType::OneShot(ref handle) => { uuid = handle.job_id; }
                }
            }
            self_w.deref_mut().jobs_to_run.insert(uuid, job.clone());
            //Run it
            {
                let mut job = job.clone();
                job.run(self.clone()).await;
            }
        }
        Ok(())
    }

    /// Remove a job from the `JobScheduler`
    ///
    /// ```rust,ignore
    /// use tokio_cron_scheduler::{Job, JobScheduler, JobToRun};
    /// let mut sched = JobScheduler::new();
    /// let job_id = sched.add(Job::new("1/10 * * * * *".parse().unwrap(), || {
    ///     println!("I get executed every 10 seconds!");
    /// }));
    /// sched.remove(job_id);
    /// ```
    async fn remove(&mut self, to_be_removed: &Uuid) -> Result<(), JobError> {
        {
            let copy = self.clone();
            let mut ws = copy.write().await;
            let job = ws.jobs_to_run
                        .get_mut(to_be_removed)
                        .ok_or(JobSchedulerError::InvalidArgument("Tried to remove value not in scheduler".to_string()))
                        .map_err(|err| { JobError::WrappedError(err.to_string()) })?;
            let mut write_guard = job.write().await;
            match *write_guard
            {
                JobType::CronJob(ref mut handle)
              | JobType::Repeated(ref mut handle)
              | JobType::OneShot(ref mut handle) => { handle.stopped = true; },
            };
        }
        Ok(())
    }

    async fn clean(&mut self) -> Result<(), JobError> {
        let mut ws = self.write().await;
        let mut retained = HashMap::new();
        for (_, job) in ws.jobs_to_run.iter_mut()
        {
            enumerate_removal(job, &mut retained).await;
        }

        ws.jobs_to_run = retained;
        Ok(())
    }
}
