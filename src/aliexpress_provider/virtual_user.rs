use crate::log::Log;
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use chrono::{Duration, NaiveDateTime, Utc};
use rand::Rng;
use std::ops::Sub;
use ureq::{Agent, Response};
use url::Url;

pub struct ImmutableResponse {
    pub url: Url,
    pub status: u16,
    pub body: String,
}

pub struct VirtualUser {
    agent: Agent,
    last_get_time: Option<NaiveDateTime>,
    thread_rng: rand::rngs::ThreadRng,
    idle_time_lower: Duration,
    idle_time_upper: Duration,
}

impl VirtualUser {
    pub fn new_with_defaults() -> Self {
        return Self::new(Duration::seconds(15), Duration::seconds(20)).unwrap();
    }

    pub fn new(idle_time_lower: Duration, idle_time_upper: Duration) -> BoxResult<Self> {
        if idle_time_lower > idle_time_upper {
            return UnexpectedError::new_as_box_result(
                "idle_time_lower must be less than idle_time_upper",
            );
        }
        let agent: Agent = ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(5))
            .timeout_write(std::time::Duration::from_secs(5))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.3 Safari/605.1.15")
            .build();
        let last_get_time = None;
        let thread_rng = rand::thread_rng();
        let instance = VirtualUser {
            agent,
            last_get_time,
            thread_rng,
            idle_time_lower,
            idle_time_upper,
        };
        return Ok(instance);
    }

    pub fn get(&mut self, url: &Url) -> PullResult<ImmutableResponse> {
        static STEP: &str = "get";
        match self.wait_as_a_user() {
            Err(e) => {
                return PullError::from_step(STEP, PullErrorKind::Http)
                    .set_message(e.to_string().as_str())
                    .set_http_url(url)
                    .to_result()
            }
            Ok(_) => {}
        }
        let req = self
            .agent
            .get(url.as_str())
            .set(
                "accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .set("accept-language", "en-US,en;q=0.9")
            .set(
                "cookie",
                "aep_usuc_f=site=glo&c_tp=USD&s_locale=en_US&b_locale=en_US;",
                // "aep_usuc_f=site=glo&c_tp=USD&s_locale=en_US&region=US&b_locale=en_US;",
            );
        let res = match req.call() {
            Err(e) => {
                return PullError::from_step(STEP, PullErrorKind::Http)
                    .set_message(e.to_string().as_str())
                    .set_http_url(url)
                    .to_result()
            }
            Ok(v) => v,
        };
        let status = res.status();
        let body = match res.into_string() {
            Err(e) => {
                return PullError::from_step(STEP, PullErrorKind::Http)
                    .set_message(e.to_string().as_str())
                    .set_http_context(url, status, "")
                    .to_result()
            }
            Ok(v) => v,
        };
        let immutable_res = ImmutableResponse {
            url: url.clone(),
            status: status,
            body: body,
        };
        return Ok(immutable_res);
    }

    fn wait_as_a_user(&mut self) -> BoxResult<()> {
        let now = Utc::now().naive_utc();
        let last_time = match self.last_get_time {
            Some(v) => v,
            None => {
                self.last_get_time = Some(now);
                return Ok(());
            }
        };
        let elapsed = now - last_time;
        if elapsed >= self.idle_time_lower {
            self.last_get_time = Some(now);
            return Ok(());
        }
        let random_duration = self.thread_rng.gen_range(
            self.idle_time_lower.num_milliseconds()..self.idle_time_upper.num_milliseconds(),
        );
        std::thread::sleep(std::time::Duration::from_millis(random_duration as u64));
        self.last_get_time = Some(Utc::now().naive_utc());
        return Ok(());
    }
}
