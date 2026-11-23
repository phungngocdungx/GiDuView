//! Hosting: let UltraViewer clients view/control this computer.
//! Uses native macOS ScreenCaptureKit + CoreGraphics backend on Apple platforms.

pub mod backend;
pub mod encode;
pub mod layout;
pub mod login;
pub mod runner;
pub mod session;
pub mod tools;

use crate::profile::Profile;
use crate::wire::Result;
use layout::Selection;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendChoice {
    #[default]
    Auto,
    Native,
    X11,
    Wayland,
}

/// Hosting options.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    /// Approve every verified login without asking: Some(true) = control, Some(false) = view only.
    pub auto_approve: Option<bool>,
    pub selection: Selection,
    /// Share the clipboard with the viewer.
    pub clipboard: bool,
    pub backend: BackendChoice,
    /// Where received files go.
    pub download_dir: Option<PathBuf>,
}

pub const USAGE: &str = "giduview host [--auto-approve control|view] [--monitor N|all] [--clipboard] \
                         [--download-dir PATH]";

impl Options {
    pub fn parse(args: &[String]) -> Result<Self> {
        let mut o = Options::default();
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            let mut value = |name: &str| it.next().cloned().ok_or_else(|| format!("{name} needs a value\nusage: {USAGE}"));
            match arg.as_str() {
                "--auto-approve" => o.auto_approve = Some(match value(arg)?.as_str() {
                    "control" => true,
                    "view" => false,
                    v => return Err(format!("--auto-approve takes control or view, not {v:?}").into()),
                }),
                "--monitor" => o.selection = Selection::parse(&value(arg)?)?,
                "--clipboard" => o.clipboard = true,
                "--backend" => o.backend = match value(arg)?.as_str() {
                    "auto" | "native" => BackendChoice::Native,
                    "x11" => BackendChoice::X11,
                    "wayland" => BackendChoice::Wayland,
                    v => return Err(format!("--backend takes auto, native, x11 or wayland, not {v:?}").into()),
                },
                "--download-dir" => o.download_dir = Some(PathBuf::from(value(arg)?)),
                other => return Err(format!("unknown host option {other:?}\nusage: {USAGE}").into()),
            }
        }
        Ok(o)
    }
}

/// Register, print ID/password, and serve viewers until stopped.
pub fn run(profile: Profile) -> Result<()> {
    run_with(profile, Options::default())
}

pub fn run_with(profile: Profile, opts: Options) -> Result<()> {
    let backend = crate::platform::open_platform_backend()?;
    runner::run(&profile.registration, Profile::master_password_proof()?, backend, &opts)
}
