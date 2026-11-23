//! macOS permissions helper for Screen Recording and Accessibility.

use super::input_macos::{has_accessibility_permission, open_accessibility_settings};
use super::screen_macos::{
    has_screen_recording_permission, open_screen_recording_settings,
    request_screen_recording_permission,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacOSPermissions {
    pub screen_recording: bool,
    pub accessibility: bool,
}

pub fn check_permissions() -> MacOSPermissions {
    MacOSPermissions {
        screen_recording: has_screen_recording_permission(),
        accessibility: has_accessibility_permission(),
    }
}

pub fn request_screen_permission() -> bool {
    request_screen_recording_permission()
}

pub fn open_screen_settings() {
    open_screen_recording_settings();
}

pub fn open_input_settings() {
    open_accessibility_settings();
}
