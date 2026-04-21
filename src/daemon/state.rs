use std::time::{Instant, Duration};

pub struct State {
    pub last_command: Option<String>,
    pub last_exit_code: Option<i32>,
    pub command_count: u32,
    pub ls_count: u32,
    pub last_message_time: Option<Instant>,
    pub last_message_is_discovery: bool,
    pub cooldown: Duration,
}

impl State {
    pub fn new() -> Self {
        Self {
            last_command: None,
            last_exit_code: None,
            command_count: 0,
            ls_count: 0,
            last_message_time: None,
            last_message_is_discovery: false,
            cooldown: Duration::from_secs(30), // 30 second cooldown by default
        }
    }

    pub fn should_show_companion(&self) -> bool {
        match self.last_message_time {
            Some(time) => time.elapsed() > self.cooldown,
            None => true,
        }
    }

    pub fn update_last_message_time(&mut self) {
        self.last_message_time = Some(Instant::now());
    }
}
