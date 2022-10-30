use std::borrow::Cow;
use std::time::{Duration,
                Instant};

/// Single message
type Msg = Cow<'static, str>;

/// Data related to the message bar
pub struct Msgs {
    pub current: Option<Msg>,
    pub last_time: Instant, // the instant at which last message was shown
    pub timeout: Duration,
    pub queue: Vec<Msg>,   // messages to be shown
    pub history: Vec<Msg>, // history of messages
}

impl Msgs {
    pub fn default() -> Self {
        Msgs {
            current: None,
            last_time: Instant::now(),
            timeout: Duration::from_secs(4),
            queue: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn on_tick(&mut self) {
        if let Some(new_msg) = self.queue.pop() {
            if self.current.is_some() {
                self.history.push(std::mem::take(&mut self.current).unwrap());
            }
            self.current = Some(new_msg);
            self.last_time = Instant::now();
        } else if self.current.is_some() && self.last_time.elapsed().cmp(&self.timeout).is_ge() {
            self.history.push(std::mem::take(&mut self.current).unwrap());
        }
    }

    pub fn push(&mut self, msg: Msg) {
        self.queue.push(msg);
    }
}
