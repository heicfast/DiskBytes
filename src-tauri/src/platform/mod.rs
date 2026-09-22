//! App-side platform module: the Windows implementation of the core
//! [`Platform`](diskbytes_core::platform::Platform) seam (doc 02 §2).
//! `win.rs` is the ONLY file allowed to call windows-rs directly.

pub use diskbytes_core::platform::{DirEntryData, DirListing, KnownFolder, ListError, Platform};

#[cfg(windows)]
pub mod win;

#[cfg(target_os = "macos")]
pub mod mac;

/// The OS dispatch module: command code says `platform::os::X`, which
/// resolves to the Windows implementation on Windows and the macOS
/// implementation on macOS (both expose the same free-function surface).
#[cfg(windows)]
pub use crate::platform::win as os;

#[cfg(target_os = "macos")]
pub use crate::platform::mac as os;

/// The platform host the app registers in managed state. Command
/// signatures take this alias so one code path compiles on both OSes
/// (doc: cross-platform architecture §4, Option A).
#[cfg(windows)]
pub use crate::platform::win::WindowsPlatform as HostPlatform;

#[cfg(target_os = "macos")]
pub use crate::platform::mac::MacPlatform as HostPlatform;
