// SPDX-License-Identifier: GPL-3.0-or-later

//! Amiga `.adf` disk images: finding a file by name on an AmigaDOS disk
//! and reading it out. Written from the AmigaDOS filesystem's published
//! block layout (see docs/provenance.md and docs/format-notes.md, Amiga
//! disks).
//!
//! A disk is 512-byte blocks, every number in them a big-endian 32-bit
//! long. The root block, in the middle of the disk, holds a 72-entry hash
//! table of the files and directories at the top level; entries that share
//! a slot are chained through each header's `hash_chain`. A file header
//! lists its data blocks (up to 72, last entry first), and extension blocks
//! carry the rest. The old file system (OFS) gives each data block a
//! 24-byte header and 488 bytes of data; the fast file system (FFS) fills
//! all 512 bytes.

use crate::error::{Error, Result};

pub const BLOCK: usize = 512;
/// Double density (880 KB) and high density (1.76 MB) disks.
pub const SIZES: [usize; 2] = [901_120, 1_802_240];

const T_HEADER: u32 = 2;
const T_LIST: u32 = 16;
const ST_ROOT: u32 = 1;
const ST_USERDIR: u32 = 2;
const ST_FILE: u32 = 0xFFFF_FFFD; // -3
const HASH_SIZE: usize = 72;
const HASH_TABLE: usize = 24;
const BYTE_SIZE: usize = 324;
const NAME: usize = 432;
const HASH_CHAIN: usize = 496;
const EXTENSION: usize = 504;
const SEC_TYPE: usize = 508;
const OFS_DATA: usize = 24;
const OFS_DATA_SIZE: usize = 12;

/// Whether `data` looks like an AmigaDOS disk: a known size and a boot
/// block starting with `DOS`.
pub fn is_adf(data: &[u8]) -> bool {
    SIZES.contains(&data.len()) && data.starts_with(b"DOS")
}

/// One file on the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Path from the root, `/`-separated.
    pub path: String,
    pub size: usize,
    header: usize,
}

pub struct Disk<'a> {
    data: &'a [u8],
    ffs: bool,
}

impl<'a> Disk<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        if !is_adf(data) {
            return Err(Error::NotAnAdventure(
                "not an AmigaDOS disk image (wrong size or no DOS boot block)".into(),
            ));
        }
        Ok(Self {
            data,
            ffs: data[3] & 1 == 1,
        })
    }

    /// Whether the disk uses the fast file system.
    pub fn ffs(&self) -> bool {
        self.ffs
    }

    fn blocks(&self) -> usize {
        self.data.len() / BLOCK
    }

    fn block(&self, n: usize) -> Result<&'a [u8]> {
        if n == 0 || n >= self.blocks() {
            return Err(Error::NotAnAdventure(format!("disk block {n} is out of range")));
        }
        Ok(&self.data[n * BLOCK..(n + 1) * BLOCK])
    }

    fn long(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    fn name(b: &[u8]) -> String {
        let len = (b[NAME] as usize).min(30);
        b[NAME + 1..NAME + 1 + len]
            .iter()
            .map(|&c| char::from(c))
            .collect()
    }

    /// Every file on the disk, depth first, root entries first.
    pub fn files(&self) -> Result<Vec<Entry>> {
        let root = self.blocks() / 2;
        let b = self.block(root)?;
        if Self::long(b, 0) != T_HEADER || Self::long(b, SEC_TYPE) != ST_ROOT {
            return Err(Error::NotAnAdventure(
                "the disk has no AmigaDOS root block".into(),
            ));
        }
        let mut out = Vec::new();
        let mut seen = vec![false; self.blocks()];
        self.walk(b, "", &mut out, &mut seen)?;
        Ok(out)
    }

    fn walk(&self, dir: &[u8], prefix: &str, out: &mut Vec<Entry>, seen: &mut [bool]) -> Result<()> {
        let mut subdirs = Vec::new();
        for slot in 0..HASH_SIZE {
            let mut next = Self::long(dir, HASH_TABLE + slot * 4) as usize;
            while next != 0 {
                if next >= seen.len() || seen[next] {
                    return Err(Error::NotAnAdventure("the disk's directory chains loop".into()));
                }
                seen[next] = true;
                let h = self.block(next)?;
                let path = format!("{prefix}{}", Self::name(h));
                match Self::long(h, SEC_TYPE) {
                    ST_FILE => out.push(Entry {
                        path,
                        size: Self::long(h, BYTE_SIZE) as usize,
                        header: next,
                    }),
                    ST_USERDIR => subdirs.push((h, path)),
                    _ => {}
                }
                next = Self::long(h, HASH_CHAIN) as usize;
            }
        }
        for (h, path) in subdirs {
            self.walk(h, &format!("{path}/"), out, seen)?;
        }
        Ok(())
    }

    /// The bytes of `entry`.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(entry.size);
        let mut list = self.block(entry.header)?;
        let mut hops = 0;
        while out.len() < entry.size {
            // The table fills from its last entry down.
            for i in (0..HASH_SIZE).rev() {
                if out.len() >= entry.size {
                    break;
                }
                let n = Self::long(list, HASH_TABLE + i * 4) as usize;
                if n == 0 {
                    break;
                }
                let d = self.block(n)?;
                let chunk = if self.ffs {
                    d
                } else {
                    let used = (Self::long(d, OFS_DATA_SIZE) as usize).min(BLOCK - OFS_DATA);
                    &d[OFS_DATA..OFS_DATA + used]
                };
                let want = (entry.size - out.len()).min(chunk.len());
                out.extend_from_slice(&chunk[..want]);
            }
            if out.len() >= entry.size {
                break;
            }
            let ext = Self::long(list, EXTENSION) as usize;
            hops += 1;
            if ext == 0 || hops > self.blocks() {
                return Err(Error::NotAnAdventure(format!(
                    "{} ends after {} of {} bytes",
                    entry.path,
                    out.len(),
                    entry.size
                )));
            }
            list = self.block(ext)?;
            if Self::long(list, 0) != T_LIST {
                return Err(Error::NotAnAdventure(format!(
                    "{}: a bad extension block",
                    entry.path
                )));
            }
        }
        Ok(out)
    }
}

/// The adventure file on an Amiga disk: the first file named `adventure`
/// (any case), the shallowest one if there are several.
pub fn adventure(data: &[u8]) -> Result<Vec<u8>> {
    let disk = Disk::new(data)?;
    let files = disk.files()?;
    let found = files
        .iter()
        .filter(|e| {
            e.path
                .rsplit('/')
                .next()
                .unwrap_or("")
                .eq_ignore_ascii_case("adventure")
        })
        .min_by_key(|e| e.path.matches('/').count())
        .ok_or_else(|| Error::NotAnAdventure("no file named adventure on this Amiga disk".into()))?;
    disk.read(found)
}

/// Writes a minimal AmigaDOS disk holding `files` at the root, for tests:
/// OFS or FFS, data blocks from block 2 upward, extension blocks as needed.
/// Checksums are left zero (the reader doesn't check them).
pub fn write_test_disk(files: &[(&str, &[u8])], ffs: bool) -> Vec<u8> {
    let mut d = vec![0u8; SIZES[0]];
    d[..4].copy_from_slice(&[b'D', b'O', b'S', u8::from(ffs)]);
    let blocks = d.len() / BLOCK;
    let root = blocks / 2;
    let put = |d: &mut Vec<u8>, block: usize, at: usize, v: u32| {
        d[block * BLOCK + at..block * BLOCK + at + 4].copy_from_slice(&v.to_be_bytes());
    };
    put(&mut d, root, 0, T_HEADER);
    put(&mut d, root, 12, HASH_SIZE as u32);
    put(&mut d, root, SEC_TYPE, ST_ROOT);
    let mut free = 2usize;
    let mut alloc = || {
        let n = free;
        free += 1;
        if free == root {
            free += 1;
        }
        n
    };
    let per = if ffs { BLOCK } else { BLOCK - OFS_DATA };
    for (name, bytes) in files {
        let header = alloc();
        let chunks: Vec<&[u8]> = bytes.chunks(per).collect();
        let data_blocks: Vec<usize> = chunks.iter().map(|_| alloc()).collect();
        for (k, (&b, chunk)) in data_blocks.iter().zip(&chunks).enumerate() {
            if ffs {
                d[b * BLOCK..b * BLOCK + chunk.len()].copy_from_slice(chunk);
            } else {
                put(&mut d, b, 0, 8);
                put(&mut d, b, 4, header as u32);
                put(&mut d, b, 8, k as u32 + 1);
                put(&mut d, b, OFS_DATA_SIZE, chunk.len() as u32);
                let next = data_blocks.get(k + 1).copied().unwrap_or(0);
                put(&mut d, b, 16, next as u32);
                d[b * BLOCK + OFS_DATA..b * BLOCK + OFS_DATA + chunk.len()].copy_from_slice(chunk);
            }
        }
        // Header, then extension blocks, 72 data pointers each.
        let mut list = header;
        for (g, group) in data_blocks.chunks(HASH_SIZE).enumerate() {
            if g > 0 {
                let ext = alloc();
                put(&mut d, list, EXTENSION, ext as u32);
                list = ext;
                put(&mut d, list, 0, T_LIST);
                put(&mut d, list, 4, list as u32);
                put(&mut d, list, SEC_TYPE, ST_FILE);
            }
            for (i, &b) in group.iter().enumerate() {
                put(&mut d, list, HASH_TABLE + (HASH_SIZE - 1 - i) * 4, b as u32);
            }
            put(&mut d, list, 8, group.len() as u32);
        }
        put(&mut d, header, 0, T_HEADER);
        put(&mut d, header, 4, header as u32);
        put(&mut d, header, BYTE_SIZE, bytes.len() as u32);
        put(&mut d, header, SEC_TYPE, ST_FILE);
        put(&mut d, header, 500, root as u32);
        d[header * BLOCK + NAME] = name.len() as u8;
        d[header * BLOCK + NAME + 1..header * BLOCK + NAME + 1 + name.len()].copy_from_slice(name.as_bytes());
        // Hash slot: AmigaDOS's name hash.
        let mut h = name.len() as u32;
        for c in name.bytes() {
            h = (h * 13 + u32::from(c.to_ascii_uppercase())) & 0x7FF;
        }
        let slot = (h as usize) % HASH_SIZE;
        let at = HASH_TABLE + slot * 4;
        let prev = Disk::long(&d[root * BLOCK..(root + 1) * BLOCK], at);
        put(&mut d, header, HASH_CHAIN, prev);
        put(&mut d, root, at, header as u32);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + i / 251) as u8).collect()
    }

    #[test]
    fn reads_files_back_on_ofs_and_ffs() {
        // 164,608 bytes needs extension blocks on both (338 OFS blocks, 322 FFS).
        let big = bytes(0x28300);
        let small = bytes(1000);
        for ffs in [false, true] {
            let img = write_test_disk(&[("readme", &small), ("Adventure", &big)], ffs);
            assert!(is_adf(&img));
            let disk = Disk::new(&img).unwrap();
            assert_eq!(disk.ffs(), ffs);
            let mut names: Vec<_> = disk
                .files()
                .unwrap()
                .into_iter()
                .map(|e| (e.path, e.size))
                .collect();
            names.sort();
            assert_eq!(
                names,
                [("Adventure".to_string(), 0x28300), ("readme".to_string(), 1000)]
            );
            assert_eq!(adventure(&img).unwrap(), big, "ffs {ffs}");
        }
    }

    #[test]
    fn refuses_what_is_not_an_adventure_disk() {
        assert!(adventure(&vec![0u8; SIZES[0]]).is_err());
        assert!(adventure(&write_test_disk(&[("readme", &bytes(10))], false)).is_err());
        // A truncated file: its size claims more than its blocks hold.
        let mut img = write_test_disk(&[("adventure", &bytes(600))], false);
        let disk = Disk::new(&img).unwrap();
        let header = disk.files().unwrap()[0].header;
        img[header * BLOCK + BYTE_SIZE..header * BLOCK + BYTE_SIZE + 4]
            .copy_from_slice(&100_000u32.to_be_bytes());
        assert!(adventure(&img).is_err());
    }
}
