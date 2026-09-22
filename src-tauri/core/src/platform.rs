//! The platform seam (doc 02 §2): every OS capability the app needs,
//! behind one trait so the app crate's `src/platform/win.rs` is the ONLY
//! place that touches windows-rs (a future `platform-macos` mirrors this
//! surface). The trait lives in core (decision D12, extending D10) so the
//! scanner below it is host-testable with a test double.
//!
//! M3 scope (doc 03): `list_dir` (the scanner's engine), drive roots,
//! known folders and volume serials. Later milestones extend the trait
//! with trash / system sampling / app listing / reveal (doc 02 §2 map).

/// One directory record as produced by the enumeration engine
/// (spec §4 `FileIdFullDirectoryInformation` fields).
#[derive(Debug, Clone)]
pub struct DirEntryData {
    /// Child name in UTF-16 (not NUL-terminated in the record; owned here).
    pub name: Vec<u16>,
    /// `FILE_ATTRIBUTE_DIRECTORY`.
    pub is_dir: bool,
    /// `EndOfFile` — logical size (MFT-resident tiny files may report
    /// `on_disk == 0`; stored as-is per spec).
    pub logical: u64,
    /// `AllocationSize` — size on disk.
    pub on_disk: u64,
    /// Last-write FILETIME → Unix seconds (0 = unknown).
    pub modified: i64,
    /// Creation FILETIME → Unix seconds (0 = unknown).
    pub created: i64,
    /// The reparse tag when `FILE_ATTRIBUTE_REPARSE_POINT` is set
    /// (spec: `EaSize` carries it); `None` otherwise.
    pub reparse_tag: Option<u32>,
    /// Cloud placeholder: any of `FILE_ATTRIBUTE_OFFLINE`,
    /// `FILE_ATTRIBUTE_RECALL_ON_OPEN`,
    /// `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` (never opened/read after).
    pub cloud: bool,
    /// `FileId` from the record (`WinSxS` hardlink dedup).
    pub file_id: u64,
}

/// Why a directory listing failed (spec §4: count access-denied with up
/// to 8 sample paths; ignore vanished files silently).
#[derive(Debug, Clone)]
pub enum ListError {
    /// `STATUS_ACCESS_DENIED` / `ERROR_ACCESS_DENIED` — counted + sampled.
    AccessDenied,
    /// File/dir vanished mid-scan — ignored silently per spec.
    Vanished,
    /// Anything else, with the OS-rendered reason.
    Other(String),
}

/// The result of enumerating one directory.
#[derive(Debug, Clone)]
pub struct DirListing {
    /// Children (minus `.` / `..`, which the engine strips).
    pub entries: Vec<DirEntryData>,
    /// Failure reason when the directory could not be listed.
    pub error: Option<ListError>,
}

/// Known folders resolved once at scan start (spec §4 apps roots +
/// quick-wins env roots; `SHGetKnownFolderPath` on Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownFolder {
    /// `%LOCALAPPDATA%` (`FOLDERID_LocalAppData`).
    LocalAppData,
    /// `%APPDATA%` / Roaming (`FOLDERID_RoamingAppData`).
    RoamingAppData,
    /// `%PROGRAMDATA%` (`FOLDERID_ProgramData`).
    ProgramData,
    /// `Program Files` (`FOLDERID_ProgramFiles`).
    ProgramFiles,
    /// `Program Files (x86)` (`FOLDERID_ProgramFilesX86`).
    ProgramFilesX86,
    /// `Program Files\WindowsApps` (composed from Program Files).
    ProgramFilesWindowsApps,
    /// `%LOCALAPPDATA%\Programs` (`FOLDERID_UserProgramFiles`).
    UserPrograms,
    /// `%USERPROFILE%` (`FOLDERID_Profile`).
    Profile,
}

/// The OS capability surface (doc 02 §2). Implementations must be
/// `Send + Sync` — the scanner shares one instance across worker threads.
pub trait Platform: Send + Sync + 'static {
    /// Enumerate one directory (verbatim `\\?\` path) in a single
    /// reusable-buffer pass (the engine owns alignment and record
    /// walking). Never opens cloud-placeholder children.
    fn list_dir(&self, verbatim_dir: &str) -> DirListing;

    /// Root paths (display form, e.g. `C:\`) of every `DRIVE_FIXED`
    /// volume — the children of the This PC synthetic root (spec §4).
    fn fixed_drive_roots(&self) -> Vec<String>;

    /// Resolve a known folder to its absolute display path.
    fn known_folder(&self, folder: KnownFolder) -> Option<String>;

    /// Volume serial number of the volume containing `path`
    /// (`WinSxS` hardlink identity; `None` when unavailable).
    fn volume_serial(&self, verbatim_path: &str) -> Option<u64>;
}
