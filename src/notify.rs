use notify_rust::{Notification, Urgency};
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};

use crate::config::CaffeineConfig;
use crate::fl;

const APP_ID: &str = "com.github.cosmic-caffeine";

fn show_notification(title: &str, body: &str, urgency: Urgency, timeout: i32) {
    if !CaffeineConfig::load().show_notifications {
        return;
    }

    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .appname("Caffeine")
            .summary(title)
            .body(body)
            .icon("caffeine")
            .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
            .urgency(urgency)
            .timeout(timeout)
            .show()
        {
            warn!("Failed to show notification: {}", e);
        } else {
            debug!("Showed notification: {}", title);
        }
    });
}

pub fn notify_enabled() {
    let title = fl!("notification-enabled-title");
    let body = fl!("notification-enabled-body");
    show_notification(&title, &body, Urgency::Low, 3000);
}

pub fn notify_enabled_with_time(remaining: &str) {
    let title = fl!("notification-enabled-title");
    let body = format!("{} - {}", fl!("notification-enabled-body"), remaining);
    show_notification(&title, &body, Urgency::Low, 3000);
}

pub fn notify_disabled() {
    let title = fl!("notification-disabled-title");
    let body = fl!("notification-disabled-body");
    show_notification(&title, &body, Urgency::Low, 3000);
}

pub fn notify_timer_expired() {
    let title = fl!("notification-timer-expired-title");
    let body = fl!("notification-timer-expired-body");
    show_notification(&title, &body, Urgency::Normal, 5000);
}

pub fn notify_warning(remaining_mins: u32) {
    let title = fl!("notification-warning-title");
    let body = format!("{} {} {}", fl!("notification-warning-body"), remaining_mins, fl!("notification-minutes-left"));
    show_notification(&title, &body, Urgency::Normal, 5000);
}

pub fn notify_error(details: &str) {
    let title = fl!("notification-error-title");
    let body = format!("{} {}", fl!("notification-error-body"), details);
    show_notification(&title, &body, Urgency::Normal, 5000);
}

pub fn schedule_warning(warning_mins: u32) {
    let warning_mins = warning_mins;
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(60));
        notify_warning(warning_mins);
    });
}