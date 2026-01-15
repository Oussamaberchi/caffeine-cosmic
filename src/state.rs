use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use zbus::zvariant::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub enum TimerSelection {
    #[default]
    Infinity,
    OneHour,
    TwoHours,
    Manual,
}

impl TimerSelection {
    pub fn label(&self) -> &'static str {
        match self {
            TimerSelection::Infinity => "Infinity",
            TimerSelection::OneHour => "1 Hour",
            TimerSelection::TwoHours => "2 Hours",
            TimerSelection::Manual => "Manual",
        }
    }

    pub fn duration_secs(&self, manual_mins: Option<u64>) -> Option<u64> {
        match self {
            TimerSelection::Infinity => None,
            TimerSelection::OneHour => Some(3600),
            TimerSelection::TwoHours => Some(7200),
            TimerSelection::Manual => manual_mins.map(|m| m * 60),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub struct CaffeineState {
    pub active: bool,
    pub selection: TimerSelection,
    pub expiry_ts: i64, // -1 for None, else timestamp
}

impl CaffeineState {
    pub fn inactive() -> Self {
        Self {
            active: false,
            selection: TimerSelection::default(),
            expiry_ts: -1,
        }
    }

    pub fn active(selection: TimerSelection, expiry_ts: Option<u64>) -> Self {
        Self {
            active: true,
            selection,
            expiry_ts: expiry_ts.map(|t| t as i64).unwrap_or(-1),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn remaining_secs(&self) -> Option<u64> {
        if !self.active || self.expiry_ts == -1 {
            return None;
        }
        let ts = self.expiry_ts as u64;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();
        if ts > now {
            Some(ts - now)
        } else {
            Some(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_selection_label_returns_correct_labels() {
        assert_eq!(TimerSelection::Infinity.label(), "Infinity");
        assert_eq!(TimerSelection::OneHour.label(), "1 Hour");
        assert_eq!(TimerSelection::TwoHours.label(), "2 Hours");
        assert_eq!(TimerSelection::Manual.label(), "Manual");
    }

    #[test]
    fn timer_selection_duration_secs() {
        assert_eq!(TimerSelection::Infinity.duration_secs(None), None);
        assert_eq!(TimerSelection::OneHour.duration_secs(None), Some(3600));
        assert_eq!(TimerSelection::TwoHours.duration_secs(None), Some(7200));
        assert_eq!(TimerSelection::Manual.duration_secs(None), None);
        assert_eq!(TimerSelection::Manual.duration_secs(Some(30)), Some(1800));
        assert_eq!(TimerSelection::Manual.duration_secs(Some(60)), Some(3600));
    }

    #[test]
    fn caffeine_state_inactive() {
        let state = CaffeineState::inactive();
        assert!(!state.is_active());
        assert_eq!(state.expiry_ts, -1);
        assert_eq!(state.selection, TimerSelection::default());
    }

    #[test]
    fn caffeine_state_active_with_expiry() {
        let expiry = 1704067200; // Some fixed timestamp
        let state = CaffeineState::active(TimerSelection::OneHour, Some(expiry));
        assert!(state.is_active());
        assert_eq!(state.expiry_ts, expiry as i64);
        assert_eq!(state.selection, TimerSelection::OneHour);
    }

    #[test]
    fn caffeine_state_active_without_expiry() {
        let state = CaffeineState::active(TimerSelection::Infinity, None);
        assert!(state.is_active());
        assert_eq!(state.expiry_ts, -1);
        assert_eq!(state.selection, TimerSelection::Infinity);
    }

    #[test]
    fn caffeine_state_remaining_secs_inactive() {
        let state = CaffeineState::inactive();
        assert_eq!(state.remaining_secs(), None);
    }

    #[test]
    fn caffeine_state_remaining_secs_infinity() {
        let state = CaffeineState::active(TimerSelection::Infinity, None);
        assert_eq!(state.remaining_secs(), None);
    }

    #[test]
    fn caffeine_state_remaining_secs_future() {
        let future_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let state = CaffeineState::active(TimerSelection::OneHour, Some(future_ts));
        let remaining = state.remaining_secs();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 3500 && remaining.unwrap() <= 3600);
    }

    #[test]
    fn caffeine_state_remaining_secs_expired() {
        let past_ts = 1; // Unix epoch - definitely expired
        let state = CaffeineState::active(TimerSelection::OneHour, Some(past_ts));
        assert_eq!(state.remaining_secs(), Some(0));
    }

    #[test]
    fn timer_selection_default_is_infinity() {
        assert_eq!(TimerSelection::default(), TimerSelection::Infinity);
    }

    #[test]
    fn caffeine_state_equality() {
        let state1 = CaffeineState::inactive();
        let state2 = CaffeineState::inactive();
        assert_eq!(state1, state2);

        let state3 = CaffeineState::active(TimerSelection::OneHour, Some(1000));
        let state4 = CaffeineState::active(TimerSelection::OneHour, Some(1000));
        assert_eq!(state3, state4);

        let state5 = CaffeineState::active(TimerSelection::TwoHours, Some(1000));
        assert_ne!(state3, state5);
    }
}
