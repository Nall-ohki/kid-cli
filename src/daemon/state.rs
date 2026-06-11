use crate::daemon::stats::Stats;
use std::time::{Instant, Duration};

#[allow(dead_code)]
pub struct State {
    pub last_command: Option<String>,
    pub last_exit_code: Option<i32>,
    pub stats: Stats,
    pub last_message_time: Option<Instant>,
    pub last_message_is_discovery: bool,
    pub cooldown: Duration,
    pub active_app: Option<String>,

    // Personality state
    pub mood: String,
    pub mood_last_set: Instant,
    pub error_streak: u32,
    pub last_activity: Instant,
    pub is_sleeping: bool,
}

impl State {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_command: None,
            last_exit_code: None,
            stats: Stats::default(), 
            last_message_time: None,
            last_message_is_discovery: false,
            cooldown: Duration::from_secs(30), // 30 second cooldown by default
            active_app: None,

            mood: "neutral".to_string(),
            mood_last_set: now,
            error_streak: 0,
            last_activity: now,
            is_sleeping: false,
        }
    }

    #[allow(dead_code)]
    pub fn should_show_companion(&self) -> bool {
        match self.last_message_time {
            Some(time) => time.elapsed() > self.cooldown,
            None => true,
        }
    }

    pub fn update_last_message_time(&mut self) {
        self.last_message_time = Some(Instant::now());
    }

    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn set_mood(&mut self, mood: &str) {
        self.mood = mood.to_string();
        self.mood_last_set = Instant::now();
    }

    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = State::new();
        assert_eq!(state.mood, "neutral");
        assert_eq!(state.error_streak, 0);
        assert!(!state.is_sleeping);
        assert!(state.should_show_companion());
    }

    #[test]
    fn test_mood_transition() {
        let mut state = State::new();
        state.set_mood("energetic");
        assert_eq!(state.mood, "energetic");
    }

    #[test]
    fn test_error_streak() {
        let mut state = State::new();
        state.error_streak = 3;
        assert_eq!(state.error_streak, 3);
        state.error_streak = 0;
        assert_eq!(state.error_streak, 0);
    }

    #[test]
    fn test_activity_tracking() {
        let mut state = State::new();
        std::thread::sleep(Duration::from_millis(10));
        let idle = state.idle_duration();
        assert!(idle >= Duration::from_millis(10));
        state.record_activity();
        let idle_after = state.idle_duration();
        assert!(idle_after < idle);
    }
}
