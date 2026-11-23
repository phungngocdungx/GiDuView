//! Platform abstraction layer for GiDuView.
//! Provides native screen capture and input injection for macOS.

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

use crate::host::backend::Backend;
use crate::wire::Result;

pub fn open_platform_backend() -> Result<Backend> {
    #[cfg(target_os = "macos")]
    {
        macos::open_backend()
    }

    #[cfg(target_os = "linux")]
    {
        // Linux fallback
        Err("Linux hosting backend requires X11 or Wayland".into())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Native screen sharing is only supported on macOS and Linux".into())
    }
}
