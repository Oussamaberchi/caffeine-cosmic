use notify_rust::{Notification, Urgency};
use std::thread;
use tracing::{debug, warn};

use crate::fl;

const APP_ID: &str = "com.github.cosmic-caffeine";

pub fn notify_enabled() {
    let title = fl!("notification-enabled-title");
    let body = fl!("notification-enabled-body");

    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .appname("Caffeine")
            .summary(&title)
            .body(&body)
            .icon("caffeine")
            .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
            .urgency(Urgency::Low)
            .timeout(3000)
            .show()
        {
            warn!("Failed to show notification: {}", e);
        } else {
            debug!("Showed caffeine enabled notification");
        }
    });
}

pub fn notify_disabled() {
    let title = fl!("notification-disabled-title");
    let body = fl!("notification-disabled-body");

    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .appname("Caffeine")
            .summary(&title)
            .body(&body)
            .icon("caffeine")
            .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
            .urgency(Urgency::Low)
            .timeout(3000)
            .show()
        {
            warn!("Failed to show notification: {}", e);
        } else {
            debug!("Showed caffeine disabled notification");
        }
    });
}

pub fn notify_timer_expired() {
    let title = fl!("notification-timer-expired-title");
    let body = fl!("notification-timer-expired-body");

    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .appname("Caffeine")
            .summary(&title)
            .body(&body)
            .icon("caffeine")
            .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
            .urgency(Urgency::Normal)
            .timeout(5000)
            .show()
        {
            warn!("Failed to show notification: {}", e);
        } else {
            debug!("Showed timer expired notification");
        }
    });
}

pub fn notify_error(details: &str) {
    let title = fl!("notification-error-title");
    let body = format!("{} {}", fl!("notification-error-body"), details);

    thread::spawn(move || {
        if let Err(e) = Notification::new()
            .appname("Caffeine")
            .summary(&title)
            .body(&body)
            .icon("dialog-error")
            .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
            .urgency(Urgency::Normal)
            .timeout(5000)
            .show()
        {
            warn!("Failed to show error notification: {}", e);
        } else {
            debug!("Showed error notification");
        }
    });
}
