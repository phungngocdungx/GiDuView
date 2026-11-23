//! Screen capture on macOS using ScreenCaptureKit and CoreGraphics.
//! Supports Retina, 4K displays, multi-monitor setups, and converts frames
//! directly to bottom-up RGB555 for UltraViewer compatibility.

use crate::host::backend::Capturer;
use crate::host::layout::Rect;
use crate::wire::Result;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

impl CGRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width: w, height: h },
        }
    }
}

type CGDirectDisplayID = u32;
type CGImageRef = *const c_void;
type CGDataProviderRef = *const c_void;
type CFDataRef = *const c_void;

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
const K_CG_NULL_WINDOW_ID: u32 = 0;
const K_CG_WINDOW_IMAGE_BEST_RESOLUTION: u32 = 0x100;
const K_CG_WINDOW_IMAGE_DEFAULT: u32 = 0;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> i32;
    fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    fn CGDisplayPixelsWide(display: CGDirectDisplayID) -> usize;
    fn CGDisplayPixelsHigh(display: CGDirectDisplayID) -> usize;
    fn CGWindowListCreateImage(
        screen_bounds: CGRect,
        list_options: u32,
        window_id: u32,
        image_options: u32,
    ) -> CGImageRef;
    fn CGDisplayCreateImage(display: CGDirectDisplayID) -> CGImageRef;
    fn CGImageGetWidth(image: CGImageRef) -> usize;
    fn CGImageGetHeight(image: CGImageRef) -> usize;
    fn CGImageGetBytesPerRow(image: CGImageRef) -> usize;
    fn CGImageGetBitsPerPixel(image: CGImageRef) -> usize;
    fn CGImageGetDataProvider(image: CGImageRef) -> CGDataProviderRef;
    fn CGImageRelease(image: CGImageRef);
    fn CGDataProviderCopyData(provider: CGDataProviderRef) -> CFDataRef;
    fn CFDataGetBytePtr(data: CFDataRef) -> *const u8;
    fn CFDataGetLength(data: CFDataRef) -> isize;
    fn CFRelease(cf: *const c_void);
}

/// Checks if macOS Screen Recording permission is currently granted.
pub fn has_screen_recording_permission() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        CGPreflightScreenCaptureAccess()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Requests macOS Screen Recording permission (shows macOS system prompt).
pub fn request_screen_recording_permission() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        CGRequestScreenCaptureAccess()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Opens the macOS System Settings page for Screen Recording.
pub fn open_screen_recording_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
    }
}

/// macOS Screen Capturer implementation.
pub struct MacCapturer {
    displays_cache: Vec<Rect>,
}

impl MacCapturer {
    pub fn new() -> Self {
        MacCapturer {
            displays_cache: Vec::new(),
        }
    }
}

impl Default for MacCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl Capturer for MacCapturer {
    fn displays(&mut self) -> Result<Vec<Rect>> {
        #[cfg(target_os = "macos")]
        {
            if !has_screen_recording_permission() {
                request_screen_recording_permission();
            }

            let mut display_ids = [0u32; 16];
            let mut count = 0u32;
            let status = unsafe {
                CGGetActiveDisplayList(16, display_ids.as_mut_ptr(), &mut count)
            };
            if status != 0 || count == 0 {
                return Err("Failed to query macOS displays via CoreGraphics".into());
            }

            let mut rects = Vec::with_capacity(count as usize);
            for &id in &display_ids[..count as usize] {
                let bounds = unsafe { CGDisplayBounds(id) };
                rects.push(Rect {
                    x: bounds.origin.x as i32,
                    y: bounds.origin.y as i32,
                    width: bounds.size.width as i32,
                    height: bounds.size.height as i32,
                });
            }

            self.displays_cache = rects.clone();
            Ok(rects)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(vec![Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }])
        }
    }

    fn capture(&mut self, rect: Rect) -> Result<Vec<u8>> {
        #[cfg(target_os = "macos")]
        {
            let cg_bounds = CGRect::new(
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
            );

            let image = unsafe {
                CGWindowListCreateImage(
                    cg_bounds,
                    K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY,
                    K_CG_NULL_WINDOW_ID,
                    K_CG_WINDOW_IMAGE_DEFAULT,
                )
            };

            if image.is_null() {
                // If Screen Recording is not granted, CGWindowListCreateImage returns NULL.
                if !has_screen_recording_permission() {
                    return Err("Screen Recording permission is required. Please grant it in macOS System Settings.".into());
                }
                return Err(format!(
                    "Failed to capture screen image for rect {}x{} at ({},{})",
                    rect.width, rect.height, rect.x, rect.y
                )
                .into());
            }

            let captured_w = unsafe { CGImageGetWidth(image) };
            let captured_h = unsafe { CGImageGetHeight(image) };
            let stride = unsafe { CGImageGetBytesPerRow(image) };
            let bpp = unsafe { CGImageGetBitsPerPixel(image) };
            let provider = unsafe { CGImageGetDataProvider(image) };

            if provider.is_null() {
                unsafe { CGImageRelease(image) };
                return Err("Failed to obtain image data provider from CGImage".into());
            }

            let cf_data = unsafe { CGDataProviderCopyData(provider) };
            if cf_data.is_null() {
                unsafe { CGImageRelease(image) };
                return Err("Failed to copy image pixel data from CoreGraphics".into());
            }

            let byte_ptr = unsafe { CFDataGetBytePtr(cf_data) };
            let data_len = unsafe { CFDataGetLength(cf_data) } as usize;

            let target_w = rect.width as usize;
            let target_h = rect.height as usize;

            // Convert to bottom-up RGB555
            let rgb555 = if captured_w == target_w && captured_h == target_h {
                let slice = unsafe { std::slice::from_raw_parts(byte_ptr, data_len) };
                convert_bgra_to_rgb555_bottom_up(slice, target_w, target_h, stride, bpp)?
            } else {
                // Handle Retina scaling (e.g. captured is 2x target_w)
                let slice = unsafe { std::slice::from_raw_parts(byte_ptr, data_len) };
                scale_and_convert_to_rgb555_bottom_up(
                    slice, captured_w, captured_h, stride, bpp, target_w, target_h,
                )?
            };

            unsafe {
                CFRelease(cf_data);
                CGImageRelease(image);
            }

            Ok(rgb555)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let target_w = rect.width as usize;
            let target_h = rect.height as usize;
            Ok(vec![0u8; target_w * target_h * 2])
        }
    }
}

/// Converts macOS standard 32-bit BGRA (or RGBA) top-down pixels into bottom-up RGB555.
/// UltraViewer protocol requires 15-bit packed RGB555 (1 bit 0, 5 bits R, 5 bits G, 5 bits B),
/// with rows sent from bottom to top.
pub fn convert_bgra_to_rgb555_bottom_up(
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bpp: usize,
) -> Result<Vec<u8>> {
    let bytes_per_pixel = bpp / 8;
    if bytes_per_pixel < 3 || src.len() < stride * height {
        return Err("Invalid pixel buffer length from screen capture".into());
    }

    let mut out = vec![0u8; width * height * 2];

    for y in 0..height {
        let src_row = y * stride;
        let dst_row = (height - 1 - y) * width * 2;
        for x in 0..width {
            let px = src_row + x * bytes_per_pixel;
            // macOS CoreGraphics default is 32-bit BGRA (B=0, G=1, R=2, A=3)
            let b = src[px] as u16;
            let g = src[px + 1] as u16;
            let r = src[px + 2] as u16;

            let word = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3);
            let out_idx = dst_row + x * 2;
            out[out_idx] = (word & 0xff) as u8;
            out[out_idx + 1] = (word >> 8) as u8;
        }
    }

    Ok(out)
}

/// Scales high-DPI Retina frames (e.g. 2x) and converts to bottom-up RGB555.
pub fn scale_and_convert_to_rgb555_bottom_up(
    src: &[u8],
    src_w: usize,
    src_h: usize,
    stride: usize,
    bpp: usize,
    dst_w: usize,
    dst_h: usize,
) -> Result<Vec<u8>> {
    let bytes_per_pixel = bpp / 8;
    if bytes_per_pixel < 3 || src.len() < stride * src_h {
        return Err("Invalid pixel buffer length from screen capture".into());
    }

    let mut out = vec![0u8; dst_w * dst_h * 2];

    let sx = src_w as f64 / dst_w as f64;
    let sy = src_h as f64 / dst_h as f64;

    for dy in 0..dst_h {
        let sy_coord = ((dy as f64 * sy) as usize).min(src_h - 1);
        let src_row = sy_coord * stride;
        let dst_row = (dst_h - 1 - dy) * dst_w * 2;

        for dx in 0..dst_w {
            let sx_coord = ((dx as f64 * sx) as usize).min(src_w - 1);
            let px = src_row + sx_coord * bytes_per_pixel;

            let b = src[px] as u16;
            let g = src[px + 1] as u16;
            let r = src[px + 2] as u16;

            let word = ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3);
            let out_idx = dst_row + dx * 2;
            out[out_idx] = (word & 0xff) as u8;
            out[out_idx + 1] = (word >> 8) as u8;
        }
    }

    Ok(out)
}
