#![allow(dead_code)]
//! Standalone x86-64 signature scanner for live process memory.
//!
//! Zero external dependencies. Drop this file into any Rust project.
//!
//! # Quick start
//! ```rust
//! mod sigscan;
//! use sigscan::{ProcessImage, Pattern, SigEntry, Resolve};
//!
//! // Implement the reader trait for your memory backend
//! struct MyReader { pid: u64, driver: DriverClient }
//! impl sigscan::MemoryRead for MyReader {
//!     fn read_bytes(&self, addr: u64, buf: &mut [u8]) -> bool {
//!         self.driver.read_process_memory(self.pid, addr, buf)
//!     }
//! }
//!
//! let reader = MyReader { pid, driver };
//! let image = ProcessImage::from_remote(&reader, module_base).unwrap();
//!
//! // Pattern scan + RIP-relative resolve
//! let sig = SigEntry {
//!     name: "GLOBAL_PTR",
//!     pattern: "48 8B 05 ?? ?? ?? ?? 48 85 C0",
//!     resolve: Resolve::RipRel { disp_offset: 3, instr_len: 7 },
//! };
//! if let Some(result) = sig.scan_and_resolve(&image, &reader) {
//!     println!("{}: match=0x{:X} resolved=0x{:X}", sig.name, result.match_va, result.resolved_va);
//! }
//!
//! // Raw pattern scan on a byte slice
//! let pat = Pattern::parse("55 56 57 41 54 41 55");
//! if let Some(offset) = pat.scan_slice(&some_bytes) {
//!     println!("found at offset {offset}");
//! }
//! ```

// ── Memory reader trait ───────────────────────────────────────────────────────

/// Trait for reading memory from any source (kernel driver, ReadProcessMemory,
/// file-backed buffer, etc.).
pub trait MemoryRead {
    fn read_bytes(&self, addr: u64, buf: &mut [u8]) -> bool;
}

/// Read a `Copy` value from a remote address via any `MemoryRead` implementation.
pub fn read_val<T: Copy>(reader: &dyn MemoryRead, addr: u64) -> Option<T> {
    let mut buf = vec![0u8; std::mem::size_of::<T>()];
    if reader.read_bytes(addr, &mut buf) {
        Some(unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const T) })
    } else {
        None
    }
}

// ── PE section info (parsed from memory) ──────────────────────────────────────

pub struct SectionInfo {
    pub name: String,
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub data: Vec<u8>,
}

/// A PE image mapped in a remote process, with section data pulled into local buffers.
pub struct ProcessImage {
    pub base: u64,
    pub sections: Vec<SectionInfo>,
}

impl ProcessImage {
    /// Parse PE headers from a remote process and read all executable sections
    /// into local memory for pattern scanning.
    pub fn from_remote(reader: &dyn MemoryRead, module_base: u64) -> Option<Self> {
        // Read DOS header to find PE signature offset
        let e_lfanew: u32 = read_val(reader, module_base + 0x3C)?;
        let pe_offset = module_base + e_lfanew as u64;

        // Verify PE signature ("PE\0\0")
        let pe_sig: u32 = read_val(reader, pe_offset)?;
        if pe_sig != 0x0000_4550 {
            return None;
        }

        // COFF header starts at pe_offset + 4
        let coff = pe_offset + 4;
        let num_sections: u16 = read_val(reader, coff + 2)?;
        let optional_header_size: u16 = read_val(reader, coff + 16)?;

        // Section headers start right after the optional header
        let section_table = coff + 20 + optional_header_size as u64;

        let mut sections = Vec::new();
        for i in 0..num_sections as u64 {
            let sh = section_table + i * 40; // IMAGE_SECTION_HEADER is 40 bytes

            // Read section name (8 bytes)
            let mut name_bytes = [0u8; 8];
            if !reader.read_bytes(sh, &mut name_bytes) {
                continue;
            }
            let name = std::str::from_utf8(&name_bytes)
                .unwrap_or("????")
                .trim_end_matches('\0')
                .to_string();

            let Some(virtual_size): Option<u32> = read_val(reader, sh + 8) else {
                continue;
            };
            let Some(virtual_address): Option<u32> = read_val(reader, sh + 12) else {
                continue;
            };
            let Some(characteristics): Option<u32> = read_val(reader, sh + 36) else {
                continue;
            };

            // IMAGE_SCN_MEM_EXECUTE = 0x20000000
            // Also accept known executable section names for games that strip flags
            let is_executable =
                (characteristics & 0x2000_0000) != 0 || name == ".text" || name.starts_with(".UBX");

            if !is_executable || virtual_size == 0 {
                continue;
            }

            // Cap reads at 256 MB to avoid allocating insane buffers
            let read_size = (virtual_size as usize).min(256 * 1024 * 1024);
            let section_va = module_base + virtual_address as u64;
            let mut data = vec![0u8; read_size];

            // Read in 64 KB chunks — kernel drivers typically can't handle
            // multi-megabyte reads in a single syscall.  Failed chunks are
            // left zeroed so the rest of the section remains scannable.
            const CHUNK: usize = 0x10000;
            let mut chunks_ok = 0usize;
            let mut _chunks_total = 0usize;
            for off in (0..read_size).step_by(CHUNK) {
                _chunks_total += 1;
                let len = CHUNK.min(read_size - off);
                if reader.read_bytes(section_va + off as u64, &mut data[off..off + len]) {
                    chunks_ok += 1;
                }
                // failed chunks stay zeroed — pattern scan skips them naturally
            }
            if chunks_ok == 0 {
                continue;
            }

            sections.push(SectionInfo {
                name,
                virtual_address,
                virtual_size,
                data,
            });
        }

        Some(Self {
            base: module_base,
            sections,
        })
    }

    /// Read a little-endian `i32` at a virtual address.
    fn read_i32_local(&self, va: u64) -> Option<i32> {
        for sec in &self.sections {
            let sec_start = self.base + sec.virtual_address as u64;
            let sec_end = sec_start + sec.data.len() as u64;
            if va >= sec_start && va + 4 <= sec_end {
                let off = (va - sec_start) as usize;
                let bytes: [u8; 4] = self.data_slice(sec, off, 4)?.try_into().ok()?;
                return Some(i32::from_le_bytes(bytes));
            }
        }
        None
    }

    fn data_slice<'a>(&self, sec: &'a SectionInfo, offset: usize, len: usize) -> Option<&'a [u8]> {
        if offset + len <= sec.data.len() {
            Some(&sec.data[offset..offset + len])
        } else {
            None
        }
    }
}

// ── Resolution strategies ─────────────────────────────────────────────────────

/// How to turn a raw pattern-match VA into the final target address.
pub enum Resolve {
    /// The match address itself is the answer (e.g. a function entry point).
    Direct,
    /// RIP-relative: read a 4-byte signed displacement at `match + disp_offset`,
    /// then compute `match + instr_len + disp`.
    RipRel { disp_offset: u32, instr_len: u32 },
    /// Read a raw little-endian u32 at `match + offset` (e.g. a struct-field
    /// offset embedded as an immediate in the instruction stream).
    ReadU32 { offset: u32 },
    /// Add a signed delta to the match VA (e.g. pattern matches inside a
    /// function body and `delta` backs up to the true entry point).
    Offset { delta: i32 },
}

// ── Scan result ───────────────────────────────────────────────────────────────

pub struct ScanResult {
    /// Virtual address where the pattern matched.
    pub match_va: u64,
    /// Virtual address after applying the resolution strategy.
    pub resolved_va: u64,
    /// RVA of the match (match_va - module_base).
    pub match_rva: u32,
    /// RVA of the resolved address (resolved_va - module_base).
    pub resolved_rva: u32,
}

// ── Signature entry ───────────────────────────────────────────────────────────

/// A named signature with its resolution strategy.
pub struct SigEntry {
    pub name: &'static str,
    pub pattern: &'static str,
    pub resolve: Resolve,
}

impl SigEntry {
    /// Scan the image for this signature and resolve the target address.
    pub fn scan_and_resolve(
        &self,
        image: &ProcessImage,
        reader: &dyn MemoryRead,
    ) -> Option<ScanResult> {
        let pat = Pattern::parse(self.pattern);
        let match_va = pat.scan_process_image(image)?;
        let resolved_va = match self.resolve {
            Resolve::Direct => match_va,
            Resolve::RipRel {
                disp_offset,
                instr_len,
            } => {
                // Try local cached data first, fall back to live read
                let disp_addr = match_va + disp_offset as u64;
                let disp = image
                    .read_i32_local(disp_addr)
                    .or_else(|| read_val::<i32>(reader, disp_addr))?;
                (match_va + instr_len as u64).wrapping_add_signed(disp as i64)
            }
            Resolve::ReadU32 { offset } => {
                let read_addr = match_va + offset as u64;
                let val = image
                    .read_i32_local(read_addr)
                    .or_else(|| read_val::<i32>(reader, read_addr))?;
                val as u64
            }
            Resolve::Offset { delta } => match_va.wrapping_add_signed(delta as i64),
        };
        Some(ScanResult {
            match_va,
            resolved_va,
            match_rva: (match_va - image.base) as u32,
            resolved_rva: (resolved_va.wrapping_sub(image.base)) as u32,
        })
    }
}

// ── AOB pattern ───────────────────────────────────────────────────────────────

/// An AOB (array-of-bytes) pattern. `None` = wildcard byte (`??`), `Some(b)` = exact match.
pub struct Pattern(pub Vec<Option<u8>>);

impl Pattern {
    /// Parse an IDA-style hex string: `"48 8B 05 ?? ?? ?? ?? 48 85 C0"`.
    pub fn parse(s: &str) -> Self {
        let bytes = s
            .split_whitespace()
            .map(|tok| {
                if tok == "??" || tok == "?" {
                    None
                } else {
                    Some(u8::from_str_radix(tok, 16).expect("invalid hex token"))
                }
            })
            .collect();
        Self(bytes)
    }

    /// Scan a byte slice for the pattern. Returns the offset of the first match.
    pub fn scan_slice(&self, data: &[u8]) -> Option<usize> {
        let pat = &self.0;
        if pat.is_empty() || data.len() < pat.len() {
            return None;
        }
        'outer: for i in 0..=(data.len() - pat.len()) {
            for (j, byte) in pat.iter().enumerate() {
                if let Some(b) = byte {
                    if data[i + j] != *b {
                        continue 'outer;
                    }
                }
            }
            return Some(i);
        }
        None
    }

    /// Scan all executable sections in a process image.
    /// Returns the VA (virtual address) of the first match.
    pub fn scan_process_image(&self, image: &ProcessImage) -> Option<u64> {
        self.scan_process_image_all(image).into_iter().next()
    }

    /// Scan all executable sections. Returns every match VA.
    pub fn scan_process_image_all(&self, image: &ProcessImage) -> Vec<u64> {
        let mut results = Vec::new();
        for sec in &image.sections {
            let sec_base = image.base + sec.virtual_address as u64;
            let mut offset = 0;
            while offset + self.0.len() <= sec.data.len() {
                if let Some(rel) = self.scan_slice(&sec.data[offset..]) {
                    let va = sec_base + (offset + rel) as u64;
                    results.push(va);
                    offset += rel + 1;
                } else {
                    break;
                }
            }
        }
        results
    }

    /// Generate a pattern from raw bytes, auto-wildcarding RIP-relative and
    /// near call/jmp displacements.
    pub fn generate(bytes: &[u8]) -> Self {
        let mut pat: Vec<Option<u8>> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            // Near call (E8) or near jmp (E9): 4-byte relative displacement
            if (b == 0xE8 || b == 0xE9) && i + 5 <= bytes.len() {
                pat.push(Some(b));
                pat.extend([None, None, None, None]);
                i += 5;
                continue;
            }
            // REX prefix + MOV/LEA with RIP-relative ModRM (mod=00, rm=101)
            if matches!(b, 0x40..=0x4F) && i + 7 <= bytes.len() {
                let op = bytes[i + 1];
                let modrm = bytes[i + 2];
                if matches!(op, 0x8B | 0x8D | 0x89) && (modrm & 0xC7 == 0x05) {
                    pat.push(Some(b));
                    pat.push(Some(op));
                    pat.push(Some(modrm));
                    pat.extend([None, None, None, None]);
                    i += 7;
                    continue;
                }
            }
            // SSE/scalar-FP: mandatory prefix + 0F escape + opcode + ModRM(rip-rel) + disp32
            if matches!(b, 0xF3 | 0xF2 | 0x66)
                && i + 8 <= bytes.len()
                && bytes[i + 1] == 0x0F
                && (bytes[i + 3] & 0xC7 == 0x05)
            {
                pat.push(Some(b));
                pat.push(Some(0x0F));
                pat.push(Some(bytes[i + 2]));
                pat.push(Some(bytes[i + 3]));
                pat.extend([None, None, None, None]);
                i += 8;
                continue;
            }
            // Plain two-byte SSE without prefix: 0F + opcode + ModRM(rip-rel) + disp32
            if b == 0x0F && i + 7 <= bytes.len() {
                let op = bytes[i + 1];
                let modrm = bytes[i + 2];
                let has_modrm = matches!(
                    op,
                    0x10..=0x17
                        | 0x28..=0x2F
                        | 0x40..=0x4F
                        | 0x50..=0x5F
                        | 0x60..=0x6F
                        | 0x70..=0x7F
                        | 0xAF
                        | 0xB6
                        | 0xB7
                        | 0xBE
                        | 0xBF
                );
                if has_modrm && (modrm & 0xC7 == 0x05) {
                    pat.push(Some(b));
                    pat.push(Some(op));
                    pat.push(Some(modrm));
                    pat.extend([None, None, None, None]);
                    i += 7;
                    continue;
                }
            }
            pat.push(Some(b));
            i += 1;
        }
        Self(pat)
    }

    /// Find all RIP-relative cross-references to `target_va` across executable sections.
    /// Returns `(instr_va, resolved_va)` for every match.
    pub fn find_xrefs(image: &ProcessImage, target_va: u64) -> Vec<(u64, u64)> {
        let mut results = Vec::new();
        for sec in &image.sections {
            let sec_base = image.base + sec.virtual_address as u64;
            let data = &sec.data;

            for d in 3..data.len().saturating_sub(3) {
                let disp = i32::from_le_bytes([data[d], data[d + 1], data[d + 2], data[d + 3]]);
                let rip = sec_base + d as u64 + 4;
                let resolved = rip.wrapping_add_signed(disp as i64);

                if resolved != target_va {
                    continue;
                }

                let rex = data[d - 3];
                let op = data[d - 2];
                let modrm = data[d - 1];
                if rex & 0xF0 != 0x40 {
                    continue;
                }
                if !matches!(op, 0x8B | 0x8D | 0x89 | 0x83 | 0xFF | 0x01 | 0x03) {
                    continue;
                }
                if modrm & 0xC7 != 0x05 {
                    continue;
                }

                let instr_va = sec_base + (d as u64 - 3);
                results.push((instr_va, resolved));
            }
        }
        results
    }

    /// Human-readable IDA-style hex string.
    pub fn to_ida_string(&self) -> String {
        self.0
            .iter()
            .map(|b| match b {
                Some(v) => format!("{v:02X}"),
                None => "??".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
