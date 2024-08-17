use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::common::ResultExt;

#[derive(Serialize, Deserialize, Clone)]
pub enum NotificationLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Success,
}

const NOTIFICATION_PREFIX: &str = "@=@=@=>";

#[derive(Serialize, Deserialize, Clone)]
pub struct Notification {
    message: String,
    level: NotificationLevel,
}

impl Notification {
    pub fn build(message: &str, level: NotificationLevel) -> Result<String> {
        let notification = Notification {
            message: message.to_string(),
            level,
        };
        serde_json::to_string(&notification).map_anyhow_err()
    }

    pub fn publish(message: &str, notification_type: NotificationLevel) {
        let n = Notification::build(message, notification_type).unwrap();
        println!("{NOTIFICATION_PREFIX}{n}");
    }

    pub fn read(notification: &str) -> Option<String> {
        if notification.starts_with(NOTIFICATION_PREFIX) {
            let notification = notification.replace(NOTIFICATION_PREFIX, "");
            Some(notification)
        } else {
            None
        }
    }
}
