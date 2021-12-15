# JACT (Just Async Chron Tokio)

Use cron-like scheduling in an async tokio environment.
Also schedule tasks at an instant or repeat them at a fixed duration.

Inspired by https://github.com/mvniekerk/tokio-chron-scheduler

## Usage

Creating a schedule for a job is done using the `FromStr` impl for the
`Schedule` type of the [cron](https://github.com/zslayton/cron) library.

The scheduling format is as follows:

```text
sec   min   hour   day of month   month   day of week   year
*     *     *      *              *       *             *
```

Time is specified for `UTC` and not your local timezone. Note that the year may
be omitted.

Comma separated values such as `5,8,10` represent more than one time value. So
for example, a schedule of `0 2,14,26 * * * *` would execute on the 2nd, 14th,
and 26th minute of every hour.

Ranges can be specified with a dash. A schedule of `0 0 * 5-10 * *` would
execute once per hour but only on day 5 through 10 of the month.

Day of the week can be specified as an abbreviation or the full name. A
schedule of `0 0 6 * * Sun,Sat` would execute at 6am on Sunday and Saturday.

#New User Note
This library was forked and updated from another one.  It was forked to try and improve
the code and make it more predictable.  The previous code was afflicted with synchronous
RwLocks that had the potential to cause threads to grind to a halt.  In addition, some 
design choices force complicated (and in some places erroneous) code.  One of those design
choices was to try and expose the underlying RwLock to the user, while also trying to lock
the RwLock "under the cover" so to speak.  This can lead to code that can trip over itself
and deadlock in unfortunate ways.

These issues have been tackled in this iteration of the code.  However, we have not started
to force the user to lock the RwLock themselves, though for Sync and Send, we do expose the
Arc Lock to the user.  All of this is to say, you can freely call the scheduler member 
functions without locking anything at all.  However, if you use the scheduler in your
asynchronous job for any reason, you need to lock it with the appropriate lock.

I'm pretty sure the type system will force you to do that anyway, but I haven't played with
it yet so I'm not sure.

## The following is not the current state of things

A simple usage example:

```rust
use Jact::{JobScheduler, JobToRun, Job};

#[tokio::main]
async fn main() {
    let mut sched = JobScheduler::new();

  
  
    sched.add(Job::new("1/10 * * * * *", |uuid, l| {
        println!("I run every 10 seconds");
    }).unwrap());

    sched.add(Job::new_async("1/7 * * * * *", |uuid, l| Box::pin( async {
        println!("I run async every 7 seconds");
    })).unwrap());

    sched.add(Job::new("1/30 * * * * *", |uuid, l| {
        println!("I run every 30 seconds");
    }).unwrap());
  
    sched.add(
      Job::new_one_shot(Duration::from_secs(18), |_uuid, _l| {
        println!("{:?} I'm only run once", chrono::Utc::now());
      }).unwrap()
    );

    let jj = Job::new_repeated(Duration::from_secs(8), |_uuid, _l| {
      println!("{:?} I'm repeated every 8 seconds", chrono::Utc::now());
    }).unwrap();
    sched.add(jj);


    let five_s_job = Job::new("1/5 * * * * *", |_uuid, _l| {
      println!("{:?} I run every 5 seconds", chrono::Utc::now());
    })
            .unwrap();
    sched.add(five_s_job);
  
    let four_s_job_async = Job::new_async("1/4 * * * * *", |_uuid, _l| Box::pin(async move {
      println!("{:?} I run async every 4 seconds", chrono::Utc::now());
    })).unwrap();
    sched.add(four_s_job_async);
  
    sched.add(
      Job::new("1/30 * * * * *", |_uuid, _l| {
        println!("{:?} I run every 30 seconds", chrono::Utc::now());
      })
              .unwrap(),
    );
  
    sched.add(
      Job::new_one_shot(Duration::from_secs(18), |_uuid, _l| {
        println!("{:?} I'm only run once", chrono::Utc::now());
      }).unwrap()
    );
  
    sched.add(
      Job::new_one_shot_async(Duration::from_secs(16), |_uuid, _l| Box::pin( async move {
        println!("{:?} I'm only run once async", chrono::Utc::now());
      })).unwrap()
    );
  
    let jj = Job::new_repeated(Duration::from_secs(8), |_uuid, _l| {
      println!("{:?} I'm repeated every 8 seconds", chrono::Utc::now());
    }).unwrap();
    sched.add(jj);
  
    let jja = Job::new_repeated_async(Duration::from_secs(7), |_uuid, _l| Box::pin(async move {
      println!("{:?} I'm repeated async every 7 seconds", chrono::Utc::now());
    })).unwrap();
    sched.add(jja);

    sched.start().await;
}
```

## Similar Libraries

* [job_scheduler](https://github.com/lholden/job_scheduler) The crate that inspired this one
* [cron](https://github.com/zslayton/cron) the cron expression parser we use.
* [schedule-rs](https://github.com/mehcode/schedule-rs) is a similar rust library that implements it's own cron expression parser.

## License

TokioCronScheduler is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

Please see the [CONTRIBUTING](CONTRIBUTING.md) file for more information.
