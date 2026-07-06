use crate::tools::sigscanner::{
    MemoryRead, Pattern, ProcessImage, Resolve, SigEntry, read_val,
};
use libmem::Process;
use std::sync::OnceLock;

/// Field offsets inside the position component struct. These have stayed stable
/// across game patches even when the static global RVAs change.
pub const POS_X_OFFSET: usize = 0x3E0;
pub const POS_Y_OFFSET: usize = 0x3E4;
pub const ACCEL_OFFSET: usize = 0x3F4;
pub const FORWARD_CHAIN: &[usize] = &[0x2A0, 0x8C0];
pub const DIRECTIONAL_CHAIN: &[usize] = &[0xB00, 0xDC0];

/// Legacy static RVAs from v1.1.4 (September 2025 build).
pub const LEGACY_POSITION_GLOBAL: usize = 0x29EBB00;
pub const LEGACY_FORWARD_GLOBAL: usize = 0x5D95520;
pub const LEGACY_DIRECTIONAL_GLOBAL: usize = 0x5D954A0;

#[derive(Debug, Clone)]
pub struct GameMappings {
    pub position_global_rva: usize,
    pub forward_global_rva: usize,
    pub directional_global_rva: usize,
}

static RESOLVED_MAPPINGS: OnceLock<GameMappings> = OnceLock::new();

pub fn mappings() -> &'static GameMappings {
    RESOLVED_MAPPINGS
        .get()
        .expect("memory mappings were not resolved; call resolve_mappings() first")
}

pub fn resolve_mappings(process: &Process, base: usize) -> Result<&'static GameMappings, String> {
    if let Some(existing) = RESOLVED_MAPPINGS.get() {
        return Ok(existing);
    }

    let reader = ProcessReader { process };
    let image = ProcessImage::from_remote(&reader, base as u64)
        .ok_or_else(|| "failed to read GoWR.exe PE image for signature scanning".to_string())?;

    println!("Scanning GoWR.exe for position globals (this may take a few seconds)...");

    let mappings = discover_mappings(process, base, &image, &reader)?;
    println!(
        "Resolved offsets:\n  position global: 0x{:X}\n  forward global:  0x{:X}\n  directional global: 0x{:X}",
        mappings.position_global_rva, mappings.forward_global_rva, mappings.directional_global_rva
    );

    RESOLVED_MAPPINGS
        .set(mappings)
        .map_err(|_| "mappings already resolved".to_string())?;

    Ok(RESOLVED_MAPPINGS.get().unwrap())
}

pub fn scan_report(process: &Process, base: usize) -> Result<(), String> {
    let reader = ProcessReader { process };
    let image = ProcessImage::from_remote(&reader, base as u64)
        .ok_or_else(|| "failed to read GoWR.exe PE image".to_string())?;

    println!("=== GoWR Memory Scan Report ===");
    println!("Module base: 0x{base:X}");

    let legacy = GameMappings {
        position_global_rva: LEGACY_POSITION_GLOBAL,
        forward_global_rva: LEGACY_FORWARD_GLOBAL,
        directional_global_rva: LEGACY_DIRECTIONAL_GLOBAL,
    };

    println!("\n--- Legacy offsets (v1.1.4) ---");
    print_mapping_status(process, base, &legacy);

    if let Ok(found) = discover_mappings(process, base, &image, &reader) {
        println!("\n--- Auto-discovered offsets ---");
        print_mapping_status(process, base, &found);
    } else {
        println!("\nAuto-discovery did not find valid mappings.");
        println!("Stand still as Kratos in-game, then re-run with the game loaded.");
    }

    println!("\n--- Player global signature ---");
    let player_sig = SigEntry {
        name: "goPlayer__sm_pPlayer",
        pattern: "48 8B 05 ?? ?? ?? ?? 48 85 C0 0F 84 62 03 00 00 48 8B 78 08",
        resolve: Resolve::RipRel {
            disp_offset: 3,
            instr_len: 7,
        },
    };
    if let Some(result) = player_sig.scan_and_resolve(&image, &reader) {
        println!(
            "  match RVA: 0x{:X}, global RVA: 0x{:X}",
            result.match_rva, result.resolved_rva
        );
        if let Some(player_ptr) = read_val::<usize>(&reader, result.resolved_va) {
            println!("  *global = 0x{player_ptr:X}");
        }
    } else {
        println!("  signature not found (game build may differ)");
    }

    Ok(())
}

fn print_mapping_status(process: &Process, base: usize, mappings: &GameMappings) {
    let pos = validate_position_global(process, base, mappings.position_global_rva);
    println!(
        "  position 0x{:X} -> {}",
        mappings.position_global_rva,
        format_validation(&pos)
    );
    if let Some(y) = pos.y {
        println!("    Y sample: {y:.3}");
    }

    let fwd = validate_pointer_chain(process, base, mappings.forward_global_rva, FORWARD_CHAIN);
    println!(
        "  forward  0x{:X} -> {}",
        mappings.forward_global_rva,
        format_validation(&fwd)
    );

    let dir = validate_pointer_chain(
        process,
        base,
        mappings.directional_global_rva,
        DIRECTIONAL_CHAIN,
    );
    println!(
        "  directional 0x{:X} -> {}",
        mappings.directional_global_rva,
        format_validation(&dir)
    );
}

fn format_validation(result: &ValidationResult) -> String {
    if result.valid {
        "OK".to_string()
    } else {
        result.reason.clone().unwrap_or_else(|| "invalid".to_string())
    }
}

#[derive(Debug)]
struct ValidationResult {
    valid: bool,
    reason: Option<String>,
    y: Option<f32>,
}

fn validate_position_global(process: &Process, base: usize, global_rva: usize) -> ValidationResult {
    let Some(ptr) = read_memory::<usize>(process, base + global_rva) else {
        return ValidationResult {
            valid: false,
            reason: Some("failed to read global pointer".into()),
            y: None,
        };
    };

    if !is_plausible_pointer(ptr) {
        return ValidationResult {
            valid: false,
            reason: Some(format!("null or invalid pointer (0x{ptr:X})")),
            y: None,
        };
    }

    let x = read_memory::<f32>(process, ptr + POS_X_OFFSET);
    let y = read_memory::<f32>(process, ptr + POS_Y_OFFSET);
    let accel = read_memory::<f32>(process, ptr + ACCEL_OFFSET);

    if !is_plausible_coord(x) || !is_plausible_coord(y) {
        return ValidationResult {
            valid: false,
            reason: Some(format!(
                "implausible coords (x={:?}, y={:?})",
                x, y
            )),
            y,
        };
    }

    if !accel.map(is_finite_f32).unwrap_or(false) {
        return ValidationResult {
            valid: false,
            reason: Some(format!("implausible acceleration ({accel:?})")),
            y,
        };
    }

    ValidationResult {
        valid: true,
        reason: None,
        y,
    }
}

fn validate_pointer_chain(
    process: &Process,
    base: usize,
    global_rva: usize,
    chain: &[usize],
) -> ValidationResult {
    let Some(addr) = resolve_chain(process, base + global_rva, chain) else {
        return ValidationResult {
            valid: false,
            reason: Some("pointer chain failed".into()),
            y: None,
        };
    };

    let value = read_memory::<f32>(process, addr);
    if !is_plausible_coord(value) {
        return ValidationResult {
            valid: false,
            reason: Some(format!("implausible float at 0x{addr:X} ({value:?})")),
            y: value,
        };
    }

    ValidationResult {
        valid: true,
        reason: None,
        y: value,
    }
}

fn discover_mappings(
    process: &Process,
    base: usize,
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
) -> Result<GameMappings, String> {
    let legacy = GameMappings {
        position_global_rva: LEGACY_POSITION_GLOBAL,
        forward_global_rva: LEGACY_FORWARD_GLOBAL,
        directional_global_rva: LEGACY_DIRECTIONAL_GLOBAL,
    };
    if mapping_set_valid(process, base, &legacy) {
        println!("Legacy offsets still valid for this build.");
        return Ok(legacy);
    }

    let position_rva = find_position_global_rva(process, base, image, reader);
    let forward_rva = find_chain_global_rva(
        process,
        base,
        image,
        reader,
        FORWARD_CHAIN,
        validate_forward_target,
        LEGACY_FORWARD_GLOBAL,
    );
    let directional_rva = find_chain_global_rva(
        process,
        base,
        image,
        reader,
        DIRECTIONAL_CHAIN,
        validate_directional_target,
        LEGACY_DIRECTIONAL_GLOBAL,
    );

    let mappings = GameMappings {
        position_global_rva: position_rva,
        forward_global_rva: forward_rva,
        directional_global_rva: directional_rva,
    };

    if mapping_set_valid(process, base, &mappings) {
        return Ok(mappings);
    }

    Err(
        "could not resolve memory offsets for this game version; run with --scan for details"
            .into(),
    )
}

fn find_position_global_rva(
    process: &Process,
    base: usize,
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
) -> usize {
    if validate_position_global(process, base, LEGACY_POSITION_GLOBAL).valid {
        return LEGACY_POSITION_GLOBAL;
    }

    for rva in find_position_global_candidates(image, reader, process, base) {
        return rva;
    }

    brute_force_position_global(process, base).unwrap_or(LEGACY_POSITION_GLOBAL)
}

fn find_chain_global_rva(
    process: &Process,
    base: usize,
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
    chain: &[usize],
    validate_target: fn(&Process, usize) -> bool,
    fallback: usize,
) -> usize {
    if validate_pointer_chain(process, base, fallback, chain).valid {
        return fallback;
    }

    for rva in find_chain_global_candidates(image, reader, process, base, chain, validate_target) {
        return rva;
    }

    fallback
}

fn mapping_set_valid(process: &Process, base: usize, mappings: &GameMappings) -> bool {
    validate_position_global(process, base, mappings.position_global_rva).valid
        && validate_pointer_chain(process, base, mappings.forward_global_rva, FORWARD_CHAIN).valid
        && validate_pointer_chain(
            process,
            base,
            mappings.directional_global_rva,
            DIRECTIONAL_CHAIN,
        )
        .valid
}

fn find_position_global_candidates(
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
    process: &Process,
    base: usize,
) -> Vec<usize> {
    let mut candidates = Vec::new();

    // movss xmm?, [reg+0x3E4] — height field access
    let height_access = Pattern::parse("F3 0F 10 ?? E4 03 00 00");
    for match_va in height_access.scan_process_image_all(image) {
        if let Some(global_rva) = find_global_for_field_access(image, reader, match_va) {
            push_unique(&mut candidates, global_rva);
        }
    }

    // movss [reg+0x3E4], xmm? — height write
    let height_write = Pattern::parse("F3 0F 11 ?? E4 03 00 00");
    for match_va in height_write.scan_process_image_all(image) {
        if let Some(global_rva) = find_global_for_field_access(image, reader, match_va) {
            push_unique(&mut candidates, global_rva);
        }
    }

    // Known player-global signature from community CE tables.
    let player_sig = SigEntry {
        name: "goPlayer__sm_pPlayer",
        pattern: "48 8B 05 ?? ?? ?? ?? 48 85 C0 0F 84 62 03 00 00 48 8B 78 08",
        resolve: Resolve::RipRel {
            disp_offset: 3,
            instr_len: 7,
        },
    };
    if let Some(result) = player_sig.scan_and_resolve(image, reader) {
        push_unique(&mut candidates, result.resolved_rva as usize);
    }

    candidates.retain(|rva| validate_position_global(process, base, *rva).valid);
    candidates
}

fn find_chain_global_candidates(
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
    process: &Process,
    base: usize,
    chain: &[usize],
    validate_target: fn(&Process, usize) -> bool,
) -> Vec<usize> {
    let mut candidates = Vec::new();

    // Scan for RIP-relative mov rax, [global] instructions.
    let global_load = Pattern::parse("48 8B 05 ?? ?? ?? ??");
    for match_va in global_load.scan_process_image_all(image) {
        let disp_addr = match_va + 3;
        let Some(disp) = image
            .read_i32_local(disp_addr)
            .or_else(|| read_val::<i32>(reader, disp_addr))
        else {
            continue;
        };
        let global_va = match_va.wrapping_add(7).wrapping_add_signed(disp as i64);
        let global_rva = (global_va - image.base) as usize;

        if let Some(addr) = resolve_chain(process, global_rva + base, chain) {
            if validate_target(process, addr) {
                push_unique(&mut candidates, global_rva);
            }
        }
    }

    candidates
}

fn find_global_for_field_access(
    image: &ProcessImage,
    reader: &ProcessReader<'_>,
    field_access_va: u64,
) -> Option<usize> {
    // Walk backwards in the same section looking for a RIP-relative global load.
    let sec = image.sections.iter().find(|s| {
        let start = image.base + s.virtual_address as u64;
        field_access_va >= start && field_access_va < start + s.data.len() as u64
    })?;

    let sec_base = image.base + sec.virtual_address as u64;
    let rel = (field_access_va - sec_base) as usize;
    let window_start = rel.saturating_sub(0x80);
    let window = &sec.data[window_start..rel];

    for i in (0..window.len().saturating_sub(6)).rev() {
        // mov rax, [rip+disp32]
        if window[i] == 0x48 && window[i + 1] == 0x8B && (window[i + 2] & 0xC7) == 0x05 {
            let instr_va = sec_base + (window_start + i) as u64;
            let disp_addr = instr_va + 3;
            let disp = image
                .read_i32_local(disp_addr)
                .or_else(|| read_val::<i32>(reader, disp_addr))?;
            let global_va = instr_va.wrapping_add(7).wrapping_add_signed(disp as i64);
            return Some((global_va - image.base) as usize);
        }
    }

    None
}

fn brute_force_position_global(process: &Process, base: usize) -> Option<usize> {
    // Search near the legacy RVA first, then widen if needed.
    const LEGACY: usize = LEGACY_POSITION_GLOBAL;
    let ranges = [
        LEGACY.saturating_sub(0x2_000_000)..LEGACY + 0x2_000_000,
        0x20_000_000..0x40_000_000,
    ];

    let mut best: Option<(usize, u32)> = None;

    for range in ranges {
        let mut rva = range.start;
        while rva < range.end {
            if validate_position_global(process, base, rva).valid {
                let score = score_position_candidate(process, base, rva);
                match best {
                    None => best = Some((rva, score)),
                    Some((_, best_score)) if score > best_score => best = Some((rva, score)),
                    _ => {}
                }
            }
            rva += 0x8;
        }
        if best.is_some() {
            break;
        }
    }

    best.map(|(rva, _)| rva)
}

fn score_position_candidate(process: &Process, base: usize, global_rva: usize) -> u32 {
    let Some(ptr) = read_memory::<usize>(process, base + global_rva) else {
        return 0;
    };

    let mut score = 0u32;
    if let Some(y) = read_memory::<f32>(process, ptr + POS_Y_OFFSET) {
        if y.abs() > 1.0 && y.abs() < 10_000.0 {
            score += 2;
        }
    }
    if let Some(x) = read_memory::<f32>(process, ptr + POS_X_OFFSET) {
        if x.abs() > 1.0 && x.abs() < 10_000.0 {
            score += 1;
        }
    }
    score
}

fn validate_forward_target(process: &Process, addr: usize) -> bool {
    is_plausible_coord(read_memory::<f32>(process, addr))
}

fn validate_directional_target(process: &Process, addr: usize) -> bool {
    let fx = read_memory::<f32>(process, addr);
    let fz = read_memory::<f32>(process, addr + 8);
    fx.map(is_finite_f32).unwrap_or(false) && fz.map(is_finite_f32).unwrap_or(false)
}

fn resolve_chain(process: &Process, mut addr: usize, offsets: &[usize]) -> Option<usize> {
    for offset in offsets {
        let ptr = read_memory::<usize>(process, addr)?;
        if !is_plausible_pointer(ptr) {
            return None;
        }
        addr = ptr + offset;
    }
    Some(addr)
}

fn read_memory<T: Copy>(process: &Process, addr: usize) -> Option<T> {
    libmem::read_memory_ex::<T>(process, addr)
}

fn is_plausible_pointer(ptr: usize) -> bool {
    ptr >= 0x10_000 && ptr < 0x7FFF_FFFF_FFFF
}

fn is_finite_f32(v: f32) -> bool {
    v.is_finite()
}

fn is_plausible_coord(v: Option<f32>) -> bool {
    match v {
        Some(value) if value.is_finite() => value.abs() < 1_000_000.0,
        _ => false,
    }
}

fn push_unique(list: &mut Vec<usize>, value: usize) {
    if !list.contains(&value) {
        list.push(value);
    }
}

pub struct ProcessReader<'a> {
    process: &'a Process,
}

impl MemoryRead for ProcessReader<'_> {
    fn read_bytes(&self, addr: u64, buf: &mut [u8]) -> bool {
        let mut offset = 0usize;
        while offset < buf.len() {
            let read_addr = addr as usize + offset;
            let chunk_size = 8.min(buf.len() - offset);
            let Some(word) = libmem::read_memory_ex::<u64>(self.process, read_addr) else {
                return false;
            };
            let bytes = word.to_le_bytes();
            buf[offset..offset + chunk_size].copy_from_slice(&bytes[..chunk_size]);
            offset += chunk_size;
        }
        true
    }
}
