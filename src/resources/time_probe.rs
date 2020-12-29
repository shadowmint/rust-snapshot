use crate::resources::time_probe::error::TimeProbeError;
use chrono::{DateTime, Utc};
use std::thread::sleep;
use std::time::{Duration, Instant, UNIX_EPOCH};

pub struct TimeProbeConfig {
    /// The time in seconds between samples in ms.
    pub interval: u64,

    /// The amount of time to wait before checking for the next sample in ms.
    pub idle: u64,

    /// The number of samples to take before halting, or -1 to run forever.
    pub samples: i64,

    /// Artifically accelerate time for testing purposes
    pub time_scale: f32,
}

pub struct TimeProbe {
    config: TimeProbeConfig,
    reference: u128,
    moment: Instant,
    last: Instant,
    sampled: i64,
}

#[derive(Debug)]
pub struct TimeSnapshot {
    /// Time since epoc in ms
    pub timestamp: u128,

    /// Time since iterator started in ms
    pub elapsed: u128,

    /// Formatted data/time in utc
    pub utc: DateTime<Utc>,
}

impl TimeProbe {
    pub fn new(mut config: TimeProbeConfig) -> TimeProbe {
        if config.time_scale <= 0f32 {
            config.time_scale = 1f32
        }
        TimeProbe {
            config,
            moment: Instant::now(),
            last: Instant::now(),
            reference: (chrono::Local::now().timestamp() as u128) * 1000,
            sampled: 0,
        }
    }

    pub fn sync_network_time(&mut self, ntp_host: &str) -> Result<(), TimeProbeError> {
        let sntpc::NtpResult {
            sec,
            nsec: _,
            roundtrip: _,
            offset: _,
        } = sntpc::request(ntp_host, 123)?;
        self.reference = sec as u128 * 1000u128;
        self.moment = Instant::now();
        self.last = Instant::now();
        Ok(())
    }

    fn as_snapshot(&self, ms_since_spawn: u128) -> TimeSnapshot {
        let d = UNIX_EPOCH + Duration::from_millis((self.reference + ms_since_spawn) as u64);
        TimeSnapshot {
            timestamp: self.reference + ms_since_spawn,
            elapsed: ms_since_spawn,
            utc: DateTime::<Utc>::from(d),
        }
    }
}

impl Iterator for TimeProbe {
    type Item = TimeSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.config.samples > 0 && self.sampled >= self.config.samples {
            return None;
        }
        loop {
            let now = Instant::now();
            let elapsed = now - self.last;
            let elapsed_real = elapsed.as_millis();
            let elapsed_scale = (elapsed_real as f32 * self.config.time_scale).floor() as u128;
            if elapsed_scale > self.config.interval as u128 {
                self.sampled += 1;
                self.last = now;
                let since_spawn_real = (now - self.moment).as_millis();
                let since_spawn_scale =
                    (since_spawn_real as f32 * self.config.time_scale).floor() as u128;
                return Some(self.as_snapshot(since_spawn_scale));
            }
            sleep(std::time::Duration::from_millis(self.config.idle));
        }
    }
}

mod error {
    use std::error::Error;
    use std::fmt;
    use std::fmt::Display;
    use std::io;

    #[derive(Debug)]
    pub enum TimeProbeError {
        NetworkSyncFailed(String),
    }

    impl Display for TimeProbeError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TimeProbeError {}

    impl From<io::Error> for TimeProbeError {
        fn from(err: io::Error) -> Self {
            TimeProbeError::NetworkSyncFailed(format!("NTP lookup failed: {}", err))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::resources::time_probe::{TimeProbe, TimeProbeConfig, TimeSnapshot};

    #[test]
    pub fn sample_at_interval() {
        let probe = TimeProbe::new(TimeProbeConfig {
            interval: 500,
            idle: 100,
            samples: 4,
            time_scale: 1f32,
        });

        let results: Vec<TimeSnapshot> = probe.collect();
        assert_eq!(results.len(), 4);

        for (i, result) in results.iter().enumerate() {
            assert_eq!((result.elapsed / 500), (i + 1) as u128);
        }
    }

    #[test]
    pub fn sample_at_interval_with_scale() {
        let probe = TimeProbe::new(TimeProbeConfig {
            interval: 1000,
            idle: 100,
            samples: 10,
            time_scale: 5f32,
        });

        let results: Vec<TimeSnapshot> = probe.collect();
        assert_eq!(results.len(), 10);

        for (i, result) in results.iter().enumerate() {
            assert_eq!((result.elapsed / 1000), (i + 1) as u128);
        }
    }

    #[test]
    pub fn sample_at_interval_with_ntp() {
        let mut probe = TimeProbe::new(TimeProbeConfig {
            interval: 1000,
            idle: 100,
            samples: 2,
            time_scale: 1f32,
        });

        probe.sync_network_time("pool.ntp.org").unwrap();

        let results: Vec<TimeSnapshot> = probe.collect();
        assert_eq!(results.len(), 2);

        for (i, result) in results.iter().enumerate() {
            println!("{:?}", result);
            assert_eq!((result.elapsed / 1000), (i + 1) as u128);
        }
    }
}
