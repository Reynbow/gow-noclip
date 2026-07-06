use libmem::*;
use std::fmt::Write as _;

fn main() {
    let process = find_process("GoWR.exe").expect("GoWR.exe not running");
    let module = find_module_ex(&process, "GoWR.exe").expect("module not found");
    let base = module.base;

    let z = read_f32(
        &process,
        resolve_chain(&process, base + 0x5D95520, &[0x2A0, 0x8C0]).unwrap(),
    )
    .unwrap();

    let mut out = String::new();
    writeln!(out, "known Z={z}").unwrap();

    for (name, rva) in [
        ("forward", 0x5D95520usize),
        ("candidate4025920", 0x4025920),
        ("candidate27F9848", 0x27F9848),
    ] {
        let Some(ptr) = read_ptr(&process, base + rva) else { continue };
        writeln!(out, "\n{name} 0x{rva:X} -> 0x{ptr:X}").unwrap();
        for off in (0x700..0xA00).step_by(4) {
            let Some(v) = read_f32(&process, ptr + off) else { continue };
            if !v.is_finite() || v.abs() > 100_000.0 {
                continue;
            }
            if (v - z).abs() < 5.0 || (off >= 0x8B0 && off <= 0x8E0) {
                writeln!(out, "  +0x{off:03X}: {v}").unwrap();
            }
        }
        for mid in [0x2A0usize, 0x8, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x40, 0x48, 0x50, 0x58, 0x60, 0x68, 0x70, 0x78, 0x80, 0x88, 0x90, 0x98, 0xA0, 0xA8, 0xB0, 0xB8, 0xC0, 0xC8, 0xD0, 0xD8, 0xE0, 0xE8, 0xF0, 0xF8, 0x100, 0x108, 0x110, 0x118, 0x120, 0x128, 0x130, 0x138, 0x140, 0x148, 0x150, 0x158, 0x160, 0x168, 0x170, 0x178, 0x180, 0x188, 0x190, 0x198, 0x1A0, 0x1A8, 0x1B0, 0x1B8, 0x1C0, 0x1C8, 0x1D0, 0x1D8, 0x1E0, 0x1E8, 0x1F0, 0x1F8, 0x200, 0x208, 0x210, 0x218, 0x220, 0x228, 0x230, 0x238, 0x240, 0x248, 0x250, 0x258, 0x260, 0x268, 0x270, 0x278, 0x280, 0x288, 0x290, 0x298, 0x2A0] {
            let Some(mid_ptr) = read_ptr(&process, ptr + mid) else { continue };
            if mid_ptr < 0x10000 { continue; }
            writeln!(out, "  +0x{mid:X} -> 0x{mid_ptr:X}").unwrap();
            for off in (0x880..0x900).step_by(4) {
                if let Some(v) = read_f32(&process, mid_ptr + off) {
                    if v.is_finite() && (v.abs() < 100_000.0) {
                        writeln!(out, "    +0x{off:03X}: {v}").unwrap();
                    }
                }
            }
        }
    }

    println!("{out}");
}

fn resolve_chain(process: &Process, mut addr: usize, offsets: &[usize]) -> Option<usize> {
    for offset in offsets {
        addr = read_ptr(process, addr)? + offset;
    }
    Some(addr)
}

fn read_ptr(process: &Process, addr: usize) -> Option<usize> {
    read_memory_ex::<usize>(process, addr)
}

fn read_f32(process: &Process, addr: usize) -> Option<f32> {
    read_memory_ex::<f32>(process, addr)
}
