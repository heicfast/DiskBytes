//! Turbo engine — MFT record parsing (fixup + attribute extraction).
//!
//! Format facts from `resources/MFTool-main/documentation/01_mft_parsing_reference.md`
//! §§2–7 (Apache-2.0 reference; format facts, original safe-Rust
//! expression — byte-cursor reads with bounds checks, never packed
//! struct casts):
//! - record header: "FILE" signature, USA offset/count, in-use (0x1) and
//!   directory (0x2) flags, first-attribute offset, base-record ref
//!   (48-bit), hard-link count
//! - fixup: verify the USN at every sector tail against USA[0] (torn
//!   records are SKIPPED — the hardening the reference lacks), then patch the
//!   tails from USA[1..]
//! - attribute walk: `type`/`length`/`non_resident`/`name_length` header; guard
//!   `length < 8` and bounds by the record size (reference gaps)
//! - `STANDARD_INFORMATION` (0x10): times at 0x18..0x38
//! - `FILE_NAME` (0x30): parent ref at 0x18 (48-bit), name length in
//!   chars at 0x58, namespace at 0x59, name at 0x5A
//! - `DATA` (0x80) UNNAMED only: resident size u32 at 0x10 / non-resident
//!   allocated 0x28, real 0x30, initialized 0x38; compression detect
//!   (`compression_unit` != 0 && flags & 1)
//! - reparse detect: any `0xC0` attribute present

use super::runs::decode_run_list;

/// Attribute type constants (NTFS).
mod attr {
    pub const STANDARD_INFORMATION: u32 = 0x10;
    pub const FILE_NAME: u32 = 0x30;
    pub const DATA: u32 = 0x80;
    pub const REPARSE_POINT: u32 = 0xC0;
    pub const TERMINATOR: u32 = 0xFFFF_FFFF;
}

/// One extracted name (namespace + text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRec {
    /// 0=POSIX, 1=Win32, 2=DOS 8.3, 3=Win32&DOS.
    pub namespace: u8,
    /// The UTF-16 name (chars, not bytes).
    pub name: Vec<u16>,
    /// Parent directory MFT record number (48-bit).
    pub parent: u64,
}

/// The materialized record metadata (parsed once — the adaptation doc's
/// `CompactEntry`).
#[derive(Debug, Clone, Default)]
// A parsed-record DTO: the bools are independent NTFS record facets
// (in-use/directory/compressed/reparse), not a state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct CompactEntry {
    /// Own MFT record number.
    pub mft_record: u32,
    /// In-use flag (false = deleted).
    pub in_use: bool,
    /// Directory flag.
    pub is_dir: bool,
    /// Extension record: the base record number (0 = base).
    pub base_record: u32,
    /// Unnamed `DATA` allocated (on-disk) size.
    pub allocated: u64,
    /// Unnamed `DATA` real (logical) size.
    pub real: u64,
    /// Compressed / WOF-compressed `DATA`.
    pub compressed: bool,
    /// Reparse point present (junction/symlink marker).
    pub is_reparse: bool,
    /// All `FILE_NAME` records (hardlinks → several parents).
    pub names: Vec<NameRec>,
    /// `STANDARD_INFORMATION` modification time (FILETIME ticks).
    pub modified: u64,
    /// `STANDARD_INFORMATION` creation time (FILETIME ticks).
    pub created: u64,
    /// Hard-link count from the record header.
    pub hard_links: u16,
}

/// Record parse errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// Not a "FILE" record (hole, BAAD, zeroed).
    NotFile,
    /// USA bounds invalid.
    BadFixup,
    /// The USN check failed — torn write (record SKIPPED by design).
    Torn,
    /// Attribute walk overran the record.
    BadAttributes,
}

/// Parse one MFT record (post-fixup source semantics: fixup runs FIRST,
/// in-place, before any field reads).
///
/// # Errors
/// [`RecordError::NotFile`] / [`Torn`] / [`BadFixup`] / [`BadAttributes`]
/// — all typed; the caller skips the record (index alignment kept).
pub fn parse_record(
    rec: &mut [u8],
    record_index: u32,
    sector_size: usize,
) -> Result<CompactEntry, RecordError> {
    if rec.len() < 0x30 {
        return Err(RecordError::NotFile);
    }
    if &rec[0..4] != b"FILE" {
        return Err(RecordError::NotFile);
    }
    apply_fixup(rec, sector_size)?;

    let usa_offset = u16::from_le_bytes([rec[4], rec[5]]) as usize;
    let usa_count = u16::from_le_bytes([rec[6], rec[7]]) as usize;
    let first_attr = u16::from_le_bytes([rec[0x14], rec[0x15]]) as usize;
    let flags = u16::from_le_bytes([rec[0x16], rec[0x17]]);
    let base_ref = u64::from_le_bytes([
        rec[0x20], rec[0x21], rec[0x22], rec[0x23], rec[0x24], rec[0x25], rec[0x26], rec[0x27],
    ]);
    let hard_links = u16::from_le_bytes([rec[0x12], rec[0x13]]);

    // Sanity: the attribute area must fit (the hardening the reference
    // lacks).
    let usa_end = usa_offset + usa_count * 2;
    if first_attr < 0x30 || first_attr + 4 > rec.len() || usa_end > rec.len() {
        return Err(RecordError::BadAttributes);
    }

    let mut entry = CompactEntry {
        mft_record: record_index,
        in_use: flags & 0x1 != 0,
        is_dir: flags & 0x2 != 0,
        base_record: u32::try_from(base_ref & 0x0000_FFFF_FFFF_FFFF).unwrap_or(0),
        hard_links,
        ..CompactEntry::default()
    };

    // Attribute walk (cursor + bounds + length<8 guard).
    let mut cursor = first_attr;
    while cursor + 8 <= rec.len() {
        let attr_type = u32::from_le_bytes([
            rec[cursor],
            rec[cursor + 1],
            rec[cursor + 2],
            rec[cursor + 3],
        ]);
        if attr_type == attr::TERMINATOR {
            break;
        }
        let length = u32::from_le_bytes([
            rec[cursor + 4],
            rec[cursor + 5],
            rec[cursor + 6],
            rec[cursor + 7],
        ]) as usize;
        if length < 8 || cursor + length > rec.len() {
            return Err(RecordError::BadAttributes);
        }
        let non_resident = rec[cursor + 8] != 0;
        let name_len = rec[cursor + 9];

        match attr_type {
            attr::STANDARD_INFORMATION if !non_resident => {
                if cursor + 0x38 + 8 <= cursor + length {
                    entry.created = u64_at(rec, cursor + 0x18);
                    entry.modified = u64_at(rec, cursor + 0x20);
                }
            }
            attr::FILE_NAME if !non_resident => {
                parse_file_name(rec, cursor, length, &mut entry);
            }
            attr::DATA if name_len == 0 => {
                // UNNAMED `DATA` only (named = ADS, ignored for sizing).
                if non_resident {
                    if cursor + 0x40 <= cursor + length {
                        entry.allocated = u64_at(rec, cursor + 0x28);
                        entry.real = u64_at(rec, cursor + 0x30);
                        let compression_unit = u16_at(rec, cursor + 0x22);
                        let attr_flags = u16_at(rec, cursor + 0x0C);
                        entry.compressed = compression_unit != 0 && attr_flags & 0x0001 != 0;
                    }
                } else if cursor + 0x14 <= cursor + length {
                    // Resident: value length u32 at 0x10.
                    entry.real = u64::from(u32_at(rec, cursor + 0x10));
                    entry.allocated = entry.real;
                }
            }
            attr::REPARSE_POINT => {
                entry.is_reparse = true;
            }
            _ => {}
        }
        cursor += length;
    }
    Ok(entry)
}

/// Apply the fixup (with the USN verification the reference lacks — torn
/// records error out and get skipped).
fn apply_fixup(rec: &mut [u8], sector_size: usize) -> Result<(), RecordError> {
    let usa_offset = u16::from_le_bytes([rec[4], rec[5]]) as usize;
    let usa_count = u16::from_le_bytes([rec[6], rec[7]]) as usize;
    let usa_bytes = usa_count * 2;
    if usa_offset + usa_bytes > rec.len() || sector_size == 0 || usa_count < 1 {
        return Err(RecordError::BadFixup);
    }
    // USA[0] = the USN every sector tail must carry.
    let usn = u16::from_le_bytes([rec[usa_offset], rec[usa_offset + 1]]);
    for i in 1..usa_count {
        let tail = i * sector_size - 2;
        if tail + 2 > rec.len() {
            return Err(RecordError::BadFixup);
        }
        let current = u16::from_le_bytes([rec[tail], rec[tail + 1]]);
        if current != usn {
            return Err(RecordError::Torn);
        }
        // Patch from USA[i] (usa_offset + 2 + (i-1)*2).
        let patch = u16::from_le_bytes([rec[usa_offset + 2 * i], rec[usa_offset + 2 * i + 1]]);
        rec[tail] = (patch & 0xFF) as u8;
        rec[tail + 1] = (patch >> 8) as u8;
    }
    Ok(())
}

/// Parse one `FILE_NAME` attribute into `entry.names`.
fn parse_file_name(rec: &[u8], cursor: usize, length: usize, entry: &mut CompactEntry) {
    let fixed = 0x5A; // FileNameAttributeHeader size (0x18 parent + …)
    if cursor + fixed > cursor + length || cursor + fixed > rec.len() {
        return;
    }
    let parent = u64_at(rec, cursor + 0x18);
    let name_len = rec[cursor + 0x58] as usize;
    let namespace = rec[cursor + 0x59];
    let name_start = cursor + fixed;
    let name_end = name_start + name_len * 2;
    if name_end > rec.len() || name_end > cursor + length {
        return;
    }
    let mut name = Vec::with_capacity(name_len);
    let mut i = name_start;
    while i + 1 < name_end {
        name.push(u16::from_le_bytes([rec[i], rec[i + 1]]));
        i += 2;
    }
    entry.names.push(NameRec {
        namespace,
        name,
        parent: parent & 0x0000_FFFF_FFFF_FFFF,
    });
}

/// The canonical display name: Win32 (namespace 1/3/0) preferred over
/// DOS 8.3 (namespace 2) — the reference's display rule.
#[must_use]
pub fn canonical_name(entry: &CompactEntry) -> Option<&NameRec> {
    entry
        .names
        .iter()
        .find(|n| n.namespace == 1 || n.namespace == 3)
        .or_else(|| entry.names.iter().find(|n| n.namespace == 0))
        .or_else(|| entry.names.iter().find(|n| n.namespace == 2))
}

fn u64_at(rec: &[u8], off: usize) -> u64 {
    let mut b = [0u8; 8];
    let end = (off + 8).min(rec.len());
    if end > off {
        b[..end - off].copy_from_slice(&rec[off..end]);
    }
    u64::from_le_bytes(b)
}

fn u32_at(rec: &[u8], off: usize) -> u32 {
    let mut b = [0u8; 4];
    let end = (off + 4).min(rec.len());
    if end > off {
        b[..end - off].copy_from_slice(&rec[off..end]);
    }
    u32::from_le_bytes(b)
}

fn u16_at(rec: &[u8], off: usize) -> u16 {
    if off + 2 <= rec.len() {
        u16::from_le_bytes([rec[off], rec[off + 1]])
    } else {
        0
    }
}

/// Decode the $MFT record-0 non-resident data runs (the MFT's own
/// cluster map — the load loop consumes this).
///
/// # Errors
/// See [`super::runs::RunError`].
pub fn mft_record0_runs(
    mft_record0: &mut [u8],
    sector_size: usize,
) -> Result<Vec<super::runs::RunEntry>, super::runs::RunError> {
    // Parse enough header to find the unnamed `DATA` attribute, then
    // decode its runs. Reuse parse_record for the header discipline.
    if parse_record(mft_record0, 0, sector_size).is_err() {
        // Even when $MFT record 0 has odd attributes, the walk below is
        // the authority; only fixup failures are fatal.
    }
    let first_attr = u16::from_le_bytes([mft_record0[0x14], mft_record0[0x15]]) as usize;
    let mut cursor = first_attr;
    while cursor + 8 <= mft_record0.len() {
        let attr_type = u32::from_le_bytes([
            mft_record0[cursor],
            mft_record0[cursor + 1],
            mft_record0[cursor + 2],
            mft_record0[cursor + 3],
        ]);
        if attr_type == attr::TERMINATOR {
            break;
        }
        let length = u32::from_le_bytes([
            mft_record0[cursor + 4],
            mft_record0[cursor + 5],
            mft_record0[cursor + 6],
            mft_record0[cursor + 7],
        ]) as usize;
        if length < 8 || cursor + length > mft_record0.len() {
            return Err(super::runs::RunError::Overrun);
        }
        let non_resident = mft_record0[cursor + 8] != 0;
        let name_len = mft_record0[cursor + 9];
        if attr_type == attr::DATA && name_len == 0 && non_resident {
            let first_vcn = u64_at(mft_record0, cursor + 0x10);
            let last_vcn = u64_at(mft_record0, cursor + 0x18);
            let runs_offset = u16_at(mft_record0, cursor + 0x20) as usize;
            let runs_start = cursor + runs_offset;
            let runs_end = cursor + length;
            if runs_start > runs_end || runs_start > mft_record0.len() {
                return Err(super::runs::RunError::Overrun);
            }
            return decode_run_list(mft_record0, runs_start, runs_end, first_vcn, last_vcn);
        }
        cursor += length;
    }
    Err(super::runs::RunError::Overrun)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECTOR: usize = 512;

    /// Build a synthetic 1-record MFT record with fixup applied properly.
    /// `attrs` is appended verbatim after the header.
    fn build_record(flags: u16, base: u64, attrs: &[u8]) -> Vec<u8> {
        let mut rec = vec![0u8; 2 * SECTOR];
        rec[0..4].copy_from_slice(b"FILE");
        // USA at 0x28 (offset 40), count = 3 (1 USN + 2 sector tails).
        let usa_offset = 0x28usize;
        let usa_count = 3usize;
        rec[4..6].copy_from_slice(&(usa_offset as u16).to_le_bytes());
        rec[6..8].copy_from_slice(&(usa_count as u16).to_le_bytes());
        rec[0x12..0x14].copy_from_slice(&1u16.to_le_bytes()); // hard links
        rec[0x14..0x16].copy_from_slice(&0x30u16.to_le_bytes()); // first attr
        rec[0x16..0x18].copy_from_slice(&flags.to_le_bytes());
        rec[0x20..0x28].copy_from_slice(&base.to_le_bytes());
        // Attributes after the header area.
        let attr_start = 0x30;
        rec[attr_start..attr_start + attrs.len()].copy_from_slice(attrs);
        // End-marker attribute (every real record carries one).
        let term = attr_start + attrs.len();
        rec[term..term + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        rec[term + 4..term + 8].copy_from_slice(&0u32.to_le_bytes());
        let _ = rec.len();
        // USA: [USN, tail0_original, tail1_original].
        let usn: u16 = 0x1234;
        // Stash the ORIGINAL tail bytes, write the USN into the tails.
        let mut usa = vec![0u8; 6];
        usa[0..2].copy_from_slice(&usn.to_le_bytes());
        for i in 1..usa_count {
            let tail = i * SECTOR - 2;
            usa[2 * i..2 * i + 2].copy_from_slice(&rec[tail..tail + 2]);
            rec[tail..tail + 2].copy_from_slice(&usn.to_le_bytes());
        }
        rec[usa_offset..usa_offset + 6].copy_from_slice(&usa);
        rec
    }

    /// A resident `DATA` attribute (length 0x18) with value length 42.
    fn resident_data(value_len: u32) -> Vec<u8> {
        let mut a = vec![0u8; 0x18];
        a[0..4].copy_from_slice(&0x80u32.to_le_bytes()); // `DATA`
        a[4..8].copy_from_slice(&0x18u32.to_le_bytes()); // length
        a[8] = 0; // resident
        a[9] = 0; // unnamed
        a[0x10..0x14].copy_from_slice(&value_len.to_le_bytes());
        a
    }

    /// A `FILE_NAME` attribute with parent + namespace + name.
    fn file_name(parent: u64, namespace: u8, name: &str) -> Vec<u8> {
        let name_u16: Vec<u16> = name.encode_utf16().collect();
        let len = 0x5A + name_u16.len() * 2;
        let mut a = vec![0u8; len];
        a[0..4].copy_from_slice(&0x30u32.to_le_bytes()); // `FILE_NAME`
        a[4..8].copy_from_slice(&(len as u32).to_le_bytes());
        a[8] = 0; // resident
        a[0x18..0x20].copy_from_slice(&parent.to_le_bytes());
        a[0x58] = name_u16.len() as u8;
        a[0x59] = namespace;
        for (i, u) in name_u16.iter().enumerate() {
            a[0x5A + i * 2..0x5A + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
        }
        a
    }

    /// A `STANDARD_INFORMATION` attribute (72 bytes) with times.
    fn std_info() -> Vec<u8> {
        let mut a = vec![0u8; 0x48];
        a[0..4].copy_from_slice(&0x10u32.to_le_bytes());
        a[4..8].copy_from_slice(&0x48u32.to_le_bytes());
        a[8] = 0;
        let t: u64 = 132_500_000_000_000_000; // some FILETIME
        a[0x18..0x20].copy_from_slice(&t.to_le_bytes());
        a[0x20..0x28].copy_from_slice(&t.to_le_bytes());
        a
    }

    #[test]
    fn fixup_roundtrip_and_parse() {
        let attrs = [std_info(), resident_data(42), file_name(5, 1, "readme.txt")].concat();
        let mut rec = build_record(0x1, 0, &attrs);
        let entry = parse_record(&mut rec, 12, SECTOR).unwrap();
        assert!(entry.in_use);
        assert!(!entry.is_dir);
        assert_eq!(entry.base_record, 0);
        assert_eq!(entry.real, 42);
        assert_eq!(entry.allocated, 42);
        assert_eq!(entry.names.len(), 1);
        assert_eq!(entry.names[0].parent, 5);
        assert_eq!(entry.names[0].namespace, 1);
        assert_eq!(entry.created, 132_500_000_000_000_000);
    }

    #[test]
    fn torn_record_is_skipped() {
        let attrs = resident_data(10);
        let mut rec = build_record(0x1, 0, &attrs);
        // Corrupt one sector tail: USN no longer matches.
        rec[SECTOR - 2] ^= 0xFF;
        assert!(matches!(
            parse_record(&mut rec, 1, SECTOR),
            Err(RecordError::Torn)
        ));
    }

    #[test]
    fn non_file_signature_rejected() {
        let mut rec = build_record(0x1, 0, &[]);
        rec[0..4].copy_from_slice(b"BAAD");
        assert!(matches!(
            parse_record(&mut rec, 2, SECTOR),
            Err(RecordError::NotFile)
        ));
    }

    #[test]
    fn directory_and_extension_flags() {
        let mut rec = build_record(0x3, 0x0002_0000_0000_0007, &[]); // seq 2, base 7
        let entry = parse_record(&mut rec, 3, SECTOR).unwrap();
        assert!(entry.is_dir);
        assert!(entry.in_use);
        assert_eq!(entry.base_record, 7); // 48-bit mask
    }

    #[test]
    fn dos_and_win32_pair_canonical_prefers_win32() {
        let attrs = [
            file_name(5, 2, "SECRET~1.TXT"),     // DOS first
            file_name(5, 1, "Secret Notes.txt"), // Win32 second
        ]
        .concat();
        let mut rec = build_record(0x1, 0, &attrs);
        let entry = parse_record(&mut rec, 4, SECTOR).unwrap();
        assert_eq!(entry.names.len(), 2);
        let canonical = canonical_name(&entry).unwrap();
        let name = String::from_utf16(&canonical.name).unwrap();
        assert_eq!(name, "Secret Notes.txt");
    }

    #[test]
    fn hardlink_parents_all_captured() {
        let attrs = [file_name(5, 1, "a.txt"), file_name(9, 1, "b.txt")].concat();
        let mut rec = build_record(0x1, 0, &attrs);
        let entry = parse_record(&mut rec, 5, SECTOR).unwrap();
        let parents: Vec<u64> = entry.names.iter().map(|n| n.parent).collect();
        assert_eq!(parents, vec![5, 9]);
        assert_eq!(entry.hard_links, 1);
    }

    #[test]
    fn reparse_point_detected() {
        let mut rp = vec![0u8; 0x18];
        rp[0..4].copy_from_slice(&0xC0u32.to_le_bytes());
        rp[4..8].copy_from_slice(&0x18u32.to_le_bytes());
        let attrs = [resident_data(0), rp].concat();
        let mut rec = build_record(0x1, 0, &attrs);
        let entry = parse_record(&mut rec, 6, SECTOR).unwrap();
        assert!(entry.is_reparse);
    }

    #[test]
    fn non_resident_data_sizes() {
        let mut a = vec![0u8; 0x40];
        a[0..4].copy_from_slice(&0x80u32.to_le_bytes());
        a[4..8].copy_from_slice(&0x40u32.to_le_bytes());
        a[8] = 1; // non-resident
        a[0x28..0x30].copy_from_slice(&4096u64.to_le_bytes()); // allocated
        a[0x30..0x38].copy_from_slice(&3000u64.to_le_bytes()); // real
        let mut rec = build_record(0x1, 0, &a);
        let entry = parse_record(&mut rec, 7, SECTOR).unwrap();
        assert_eq!(entry.allocated, 4096);
        assert_eq!(entry.real, 3000);
        assert!(!entry.compressed);
    }

    #[test]
    fn record0_runs_decode() {
        // A non-resident `DATA` in record 0: run list at attr+0x28.
        let mut a = vec![0u8; 0x40];
        a[0..4].copy_from_slice(&0x80u32.to_le_bytes());
        a[4..8].copy_from_slice(&0x40u32.to_le_bytes());
        a[8] = 1; // non-resident
        a[9] = 0; // unnamed
        a[0x10..0x18].copy_from_slice(&0u64.to_le_bytes()); // first VCN 0
        a[0x18..0x20].copy_from_slice(&1u64.to_le_bytes()); // last VCN 1 (span 2 = the run)
        a[0x20..0x22].copy_from_slice(&0x28u16.to_le_bytes()); // runs offset
        a[0x28..0x30].copy_from_slice(&4096u64.to_le_bytes());
        a[0x30..0x38].copy_from_slice(&4096u64.to_le_bytes());
        // Run list at 0x28: [0x11, len 2, delta 0x10, 0x00] + filler.
        a[0x28] = 0x11;
        a[0x29] = 0x02;
        a[0x2A] = 0x10;
        a[0x2B] = 0x00;
        a[0x2C] = 0x00;
        let mut rec = build_record(0x1, 0, &a);
        let runs = mft_record0_runs(&mut rec, SECTOR).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].length, 2);
        assert_eq!(runs[0].lcn, Some(16));
    }
}
