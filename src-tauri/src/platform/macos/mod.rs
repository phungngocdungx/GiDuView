//! macOS platform support (ScreenCaptureKit, CoreGraphics, Accessibility).

pub mod input_macos;
pub mod permissions;
pub mod screen_macos;

use crate::host::backend::Backend;
use crate::wire::Result;

pub fn open_backend() -> Result<Backend> {
    Ok(Backend {
        name: "macOS CoreGraphics / ScreenCaptureKit",
        capturer: Box::new(screen_macos::MacCapturer::new()),
        injector: Box::new(input_macos::MacInjector::new()),
    })
}
