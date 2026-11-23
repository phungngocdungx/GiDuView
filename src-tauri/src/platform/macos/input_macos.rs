//! Input injection on macOS using CoreGraphics (CGEvent).
//! Injects mouse moves, clicks, double clicks, scroll wheel, and keyboard events.
//! Handles Windows VK to macOS CGKeyCode translation and the Mac Command vs Windows Ctrl difference.

use crate::host::backend::Injector;
use crate::input::{
    InputEvent, KEY_DOWN, KEY_UP, SYS_KEY_DOWN, SYS_KEY_UP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP,
};
use crate::wire::Result;
use std::collections::HashSet;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

type CGEventRef = *const c_void;
type CGEventSourceRef = *const c_void;
type CGKeyCode = u16;

const K_CG_HID_EVENT_TAP: u32 = 0;

// CGEventType
const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
const K_CG_EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
const K_CG_EVENT_RIGHT_MOUSE_UP: u32 = 4;
const K_CG_EVENT_MOUSE_MOVED: u32 = 5;
const K_CG_EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
const K_CG_EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_KEY_UP: u32 = 11;
const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
const K_CG_EVENT_SCROLL_WHEEL: u32 = 22;
const K_CG_EVENT_OTHER_MOUSE_DOWN: u32 = 25;
const K_CG_EVENT_OTHER_MOUSE_UP: u32 = 26;
const K_CG_EVENT_OTHER_MOUSE_DRAGGED: u32 = 27;

// CGMouseButton
const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
const K_CG_MOUSE_BUTTON_RIGHT: u32 = 1;
const K_CG_MOUSE_BUTTON_CENTER: u32 = 2;

// CGScrollEventUnit
const K_CG_SCROLL_EVENT_UNIT_LINE: u32 = 0;

// CGEventFlags
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x00100000;
const K_CG_EVENT_FLAG_MASK_SHIFT: u64 = 0x00020000;
const K_CG_EVENT_FLAG_MASK_CONTROL: u64 = 0x00040000;
const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000;
const K_CG_EVENT_FLAG_MASK_ALPHA_LOCK: u64 = 0x00010000;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn CGEventCreateMouseEvent(
        source: CGEventSourceRef,
        mouse_type: u32,
        mouse_cursor_position: CGPoint,
        mouse_button: u32,
    ) -> CGEventRef;
    fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtual_key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventCreateScrollWheelEvent2(
        source: CGEventSourceRef,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> CGEventRef;
    fn CGEventKeyboardSetUnicodeString(
        event: CGEventRef,
        string_length: usize,
        unicode_string: *const u16,
    );
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CFRelease(cf: *const c_void);
}

/// Checks if macOS Accessibility permission (required to inject input) is granted.
pub fn has_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        AXIsProcessTrusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Opens the macOS System Settings page for Accessibility.
pub fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

/// macOS input injector implementing the `Injector` trait.
pub struct MacInjector {
    last_cursor_x: f64,
    last_cursor_y: f64,
    held_keys: HashSet<CGKeyCode>,
    held_buttons: HashSet<u32>,
    active_modifiers: u64,
    /// When true, translates Windows Ctrl into Mac Command (⌘), and Windows Win into Mac Control.
    /// This makes shortcuts like Ctrl+C, Ctrl+V feel natural when controlled from Windows.
    pub map_ctrl_to_cmd: bool,
}

impl MacInjector {
    pub fn new() -> Self {
        MacInjector {
            last_cursor_x: 0.0,
            last_cursor_y: 0.0,
            held_keys: HashSet::new(),
            held_buttons: HashSet::new(),
            active_modifiers: 0,
            map_ctrl_to_cmd: true,
        }
    }

    fn post_event(&self, event: CGEventRef) {
        #[cfg(target_os = "macos")]
        if !event.is_null() {
            unsafe {
                CGEventPost(K_CG_HID_EVENT_TAP, event);
                CFRelease(event);
            }
        }
    }
}

impl Default for MacInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl Injector for MacInjector {
    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            if !has_accessibility_permission() {
                open_accessibility_settings();
                return Err("Accessibility permission is required to inject input on macOS. Please enable GiDuView in System Settings -> Privacy & Security -> Accessibility.".into());
            }

            match *event {
                InputEvent::Mouse { msg, x, y, wheel } => {
                    self.inject_mouse(msg, x, y, wheel)?;
                }
                InputEvent::Key { kind, vk, scan, ch } => {
                    self.inject_key(kind, vk, scan, ch)?;
                }
            }
        }
        let _ = event;
        Ok(())
    }

    fn release_all(&mut self) {
        #[cfg(target_os = "macos")]
        {
            // Release held mouse buttons
            let pos = CGPoint {
                x: self.last_cursor_x,
                y: self.last_cursor_y,
            };
            if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_LEFT) {
                let ev = unsafe {
                    CGEventCreateMouseEvent(
                        std::ptr::null(),
                        K_CG_EVENT_LEFT_MOUSE_UP,
                        pos,
                        K_CG_MOUSE_BUTTON_LEFT,
                    )
                };
                self.post_event(ev);
            }
            if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_RIGHT) {
                let ev = unsafe {
                    CGEventCreateMouseEvent(
                        std::ptr::null(),
                        K_CG_EVENT_RIGHT_MOUSE_UP,
                        pos,
                        K_CG_MOUSE_BUTTON_RIGHT,
                    )
                };
                self.post_event(ev);
            }
            if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_CENTER) {
                let ev = unsafe {
                    CGEventCreateMouseEvent(
                        std::ptr::null(),
                        K_CG_EVENT_OTHER_MOUSE_UP,
                        pos,
                        K_CG_MOUSE_BUTTON_CENTER,
                    )
                };
                self.post_event(ev);
            }
            self.held_buttons.clear();

            // Release held keys
            for &code in &self.held_keys {
                let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), code, false) };
                self.post_event(ev);
            }
            self.held_keys.clear();
            self.active_modifiers = 0;
        }
    }
}

impl MacInjector {
    fn inject_mouse(&mut self, msg: u32, x: i32, y: i32, wheel: i16) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            if msg == WM_MOUSEMOVE {
                self.last_cursor_x = x as f64;
                self.last_cursor_y = y as f64;
                let pos = CGPoint {
                    x: self.last_cursor_x,
                    y: self.last_cursor_y,
                };

                let event_type = if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_LEFT) {
                    K_CG_EVENT_LEFT_MOUSE_DRAGGED
                } else if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_RIGHT) {
                    K_CG_EVENT_RIGHT_MOUSE_DRAGGED
                } else if self.held_buttons.contains(&K_CG_MOUSE_BUTTON_CENTER) {
                    K_CG_EVENT_OTHER_MOUSE_DRAGGED
                } else {
                    K_CG_EVENT_MOUSE_MOVED
                };

                let ev = unsafe {
                    CGEventCreateMouseEvent(
                        std::ptr::null(),
                        event_type,
                        pos,
                        K_CG_MOUSE_BUTTON_LEFT,
                    )
                };
                self.post_event(ev);
                return Ok(());
            }

            let pos = CGPoint {
                x: self.last_cursor_x,
                y: self.last_cursor_y,
            };

            match msg {
                WM_LBUTTONDOWN => {
                    self.held_buttons.insert(K_CG_MOUSE_BUTTON_LEFT);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_LEFT_MOUSE_DOWN,
                            pos,
                            K_CG_MOUSE_BUTTON_LEFT,
                        )
                    };
                    self.post_event(ev);
                }
                WM_LBUTTONUP => {
                    self.held_buttons.remove(&K_CG_MOUSE_BUTTON_LEFT);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_LEFT_MOUSE_UP,
                            pos,
                            K_CG_MOUSE_BUTTON_LEFT,
                        )
                    };
                    self.post_event(ev);
                }
                WM_RBUTTONDOWN => {
                    self.held_buttons.insert(K_CG_MOUSE_BUTTON_RIGHT);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_RIGHT_MOUSE_DOWN,
                            pos,
                            K_CG_MOUSE_BUTTON_RIGHT,
                        )
                    };
                    self.post_event(ev);
                }
                WM_RBUTTONUP => {
                    self.held_buttons.remove(&K_CG_MOUSE_BUTTON_RIGHT);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_RIGHT_MOUSE_UP,
                            pos,
                            K_CG_MOUSE_BUTTON_RIGHT,
                        )
                    };
                    self.post_event(ev);
                }
                WM_MBUTTONDOWN => {
                    self.held_buttons.insert(K_CG_MOUSE_BUTTON_CENTER);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_OTHER_MOUSE_DOWN,
                            pos,
                            K_CG_MOUSE_BUTTON_CENTER,
                        )
                    };
                    self.post_event(ev);
                }
                WM_MBUTTONUP => {
                    self.held_buttons.remove(&K_CG_MOUSE_BUTTON_CENTER);
                    let ev = unsafe {
                        CGEventCreateMouseEvent(
                            std::ptr::null(),
                            K_CG_EVENT_OTHER_MOUSE_UP,
                            pos,
                            K_CG_MOUSE_BUTTON_CENTER,
                        )
                    };
                    self.post_event(ev);
                }
                WM_MOUSEWHEEL => {
                    // Windows standard wheel step is 120
                    let lines = (wheel as f64 / 120.0).round() as i32;
                    let ev = unsafe {
                        CGEventCreateScrollWheelEvent2(
                            std::ptr::null(),
                            K_CG_SCROLL_EVENT_UNIT_LINE,
                            1,
                            lines,
                            0,
                            0,
                        )
                    };
                    self.post_event(ev);
                }
                0x20e => {
                    // Horizontal scroll wheel
                    let lines = (wheel as f64 / 120.0).round() as i32;
                    let ev = unsafe {
                        CGEventCreateScrollWheelEvent2(
                            std::ptr::null(),
                            K_CG_SCROLL_EVENT_UNIT_LINE,
                            2,
                            0,
                            lines,
                            0,
                        )
                    };
                    self.post_event(ev);
                }
                _ => {}
            }
        }
        let _ = (msg, x, y, wheel);
        Ok(())
    }

    fn inject_key(&mut self, kind: u8, vk: u8, scan: u16, ch: u16) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let is_down = kind == KEY_DOWN || kind == SYS_KEY_DOWN;
            let is_up = kind == KEY_UP || kind == SYS_KEY_UP;

            if !is_down && !is_up {
                return Ok(());
            }

            // Translate Windows VK to macOS CGKeyCode
            let key_code = self.translate_vk(vk, scan);

            if let Some(code) = key_code {
                let is_modifier = self.update_modifier_state(code, is_down);

                let ev = if is_modifier {
                    // Flags changed event for modifier keys
                    let event = unsafe {
                        CGEventCreateKeyboardEvent(std::ptr::null(), code, is_down)
                    };
                    if !event.is_null() {
                        unsafe { CGEventSetFlags(event, self.active_modifiers) };
                    }
                    event
                } else {
                    let event = unsafe {
                        CGEventCreateKeyboardEvent(std::ptr::null(), code, is_down)
                    };
                    if !event.is_null() {
                        unsafe { CGEventSetFlags(event, self.active_modifiers) };
                        if ch > 0 && is_down {
                            unsafe {
                                CGEventKeyboardSetUnicodeString(event, 1, &ch);
                            }
                        }
                    }
                    event
                };

                if is_down {
                    self.held_keys.insert(code);
                } else {
                    self.held_keys.remove(&code);
                }

                self.post_event(ev);
            } else if ch > 0 && is_down {
                // If no direct keycode mapping exists but a Unicode character is supplied
                let ev = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), 0, true) };
                if !ev.is_null() {
                    unsafe {
                        CGEventKeyboardSetUnicodeString(ev, 1, &ch);
                    }
                    self.post_event(ev);
                    let ev_up = unsafe { CGEventCreateKeyboardEvent(std::ptr::null(), 0, false) };
                    self.post_event(ev_up);
                }
            }
        }
        let _ = (kind, vk, scan, ch);
        Ok(())
    }

    fn translate_vk(&self, vk: u8, scan: u16) -> Option<CGKeyCode> {
        let extended = scan & 0xff00 == 0xe000 || scan & 0x0100 != 0;

        // Modifier handling with optional Mac Command <-> Windows Ctrl swapping
        match vk {
            0x10 => return Some(if scan & 0xff == 0x36 { 0x3c } else { 0x38 }), // Shift
            0x11 => {
                // Windows Ctrl
                if self.map_ctrl_to_cmd {
                    // Map Windows Ctrl -> Mac Command (⌘)
                    return Some(if extended { 0x36 } else { 0x37 });
                } else {
                    // Raw Mac Control
                    return Some(if extended { 0x3e } else { 0x3b });
                }
            }
            0x12 => return Some(if extended { 0x3d } else { 0x3a }), // Alt / Option
            0x5b | 0x5c => {
                // Windows Key (LWIN/RWIN)
                if self.map_ctrl_to_cmd {
                    // Map Windows Win -> Mac Control (^)
                    return Some(if vk == 0x5c { 0x3e } else { 0x3b });
                } else {
                    // Raw Mac Command
                    return Some(if vk == 0x5c { 0x36 } else { 0x37 });
                }
            }
            0x0d => return Some(if extended { 0x4c } else { 0x24 }), // Enter / Keypad Enter
            _ => {}
        }

        VK_TO_MACOS_MAP.iter().find_map(|&(v, code)| {
            if v == vk {
                Some(code)
            } else {
                None
            }
        })
    }

    fn update_modifier_state(&mut self, code: CGKeyCode, is_down: bool) -> bool {
        let mask = match code {
            0x37 | 0x36 => K_CG_EVENT_FLAG_MASK_COMMAND,
            0x38 | 0x3c => K_CG_EVENT_FLAG_MASK_SHIFT,
            0x3b | 0x3e => K_CG_EVENT_FLAG_MASK_CONTROL,
            0x3a | 0x3d => K_CG_EVENT_FLAG_MASK_ALTERNATE,
            0x39 => K_CG_EVENT_FLAG_MASK_ALPHA_LOCK,
            _ => return false,
        };

        if is_down {
            self.active_modifiers |= mask;
        } else {
            self.active_modifiers &= !mask;
        }

        true
    }
}

/// Windows Virtual Key -> Apple hardware key position (CGKeyCode).
/// Ported from MacViewer's MacHostInputController.swift.
const VK_TO_MACOS_MAP: &[(u8, CGKeyCode)] = &[
    (0x08, 0x33), // Backspace
    (0x09, 0x30), // Tab
    (0x0c, 0x47), // Clear
    (0x13, 0x71), // Pause
    (0x14, 0x39), // CapsLock
    (0x1b, 0x35), // Escape
    (0x20, 0x31), // Space
    (0x21, 0x74), // Page Up
    (0x22, 0x79), // Page Down
    (0x23, 0x77), // End
    (0x24, 0x73), // Home
    (0x25, 0x7b), // Left Arrow
    (0x26, 0x7e), // Up Arrow
    (0x27, 0x7c), // Right Arrow
    (0x28, 0x7d), // Down Arrow
    (0x2c, 0x69), // PrintScreen
    (0x2d, 0x72), // Insert
    (0x2e, 0x75), // Delete
    // 0..9
    (0x30, 0x1d),
    (0x31, 0x12),
    (0x32, 0x13),
    (0x33, 0x14),
    (0x34, 0x15),
    (0x35, 0x17),
    (0x36, 0x16),
    (0x37, 0x1a),
    (0x38, 0x1c),
    (0x39, 0x19),
    // A..Z
    (0x41, 0x00),
    (0x42, 0x0b),
    (0x43, 0x08),
    (0x44, 0x02),
    (0x45, 0x0e),
    (0x46, 0x03),
    (0x47, 0x05),
    (0x48, 0x04),
    (0x49, 0x22),
    (0x4a, 0x26),
    (0x4b, 0x28),
    (0x4c, 0x25),
    (0x4d, 0x2e),
    (0x4e, 0x2d),
    (0x4f, 0x1f),
    (0x50, 0x23),
    (0x51, 0x0c),
    (0x52, 0x0f),
    (0x53, 0x01),
    (0x54, 0x11),
    (0x55, 0x20),
    (0x56, 0x09),
    (0x57, 0x0d),
    (0x58, 0x07),
    (0x59, 0x10),
    (0x5a, 0x06),
    // Numpad 0..9
    (0x60, 0x52),
    (0x61, 0x53),
    (0x62, 0x54),
    (0x63, 0x55),
    (0x64, 0x56),
    (0x65, 0x57),
    (0x66, 0x58),
    (0x67, 0x59),
    (0x68, 0x5b),
    (0x69, 0x5c),
    (0x6a, 0x43), // Multiply
    (0x6b, 0x45), // Add
    (0x6d, 0x4e), // Subtract
    (0x6e, 0x41), // Decimal
    (0x6f, 0x4b), // Divide
    // F1..F12
    (0x70, 0x7a),
    (0x71, 0x78),
    (0x72, 0x63),
    (0x73, 0x76),
    (0x74, 0x60),
    (0x75, 0x61),
    (0x76, 0x62),
    (0x77, 0x64),
    (0x78, 0x65),
    (0x79, 0x6d),
    (0x7a, 0x67),
    (0x7b, 0x6f),
    // Punctuation
    (0xba, 0x29), // Semicolon
    (0xbb, 0x18), // Equal
    (0xbc, 0x2b), // Comma
    (0xbd, 0x1b), // Minus
    (0xbe, 0x2f), // Period
    (0xbf, 0x2c), // Slash
    (0xc0, 0x32), // Backquote / Tilde
    (0xdb, 0x21), // BracketLeft
    (0xdc, 0x2a), // Backslash
    (0xdd, 0x1e), // BracketRight
    (0xde, 0x27), // Quote
];
