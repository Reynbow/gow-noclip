use libmem::*;

fn main() {
    let process = find_process("GoWR.exe").expect("GoWR.exe not running");
    let module = find_module_ex(&process, "GoWR.exe").expect("module not found");
    let base = module.base;

    let z_before = read_chain_f32(&process, base + 0x5D95520, &[0x2A0, 0x8C0]).unwrap();
    let y_base = resolve_chain(&process, base + 0x5D95520, &[0x2A0, 0x0]).unwrap();
    let y_before = read_f32(&process, y_base + 0x8C4).unwrap();

    println!("Before: y={y_before:.3} z={z_before:.3}");

    let z_addr = resolve_chain(&process, base + 0x5D95520, &[0x2A0, 0x8C0]).unwrap();
    let y_base = resolve_chain(&process, base + 0x5D95520, &[0x2A0, 0x0]).unwrap();
    let y_addr = y_base + 0x8C4;

    write_f32(&process, z_addr, z_before + 10.0);
    write_f32(&process, y_addr, y_before + 5.0);

    let z_after = read_f32(&process, z_addr).unwrap();
    let y_after = read_f32(&process, y_addr).unwrap();

    println!("After:  y={y_after:.3} z={z_after:.3}");
    println!("Wrote to Z=0x{z_addr:X} Y=0x{y_addr:X}");
}

fn resolve_chain(process: &Process, mut addr: usize, offsets: &[usize]) -> Option<usize> {
    for offset in offsets {
        let ptr = read_memory_ex::<usize>(process, addr)?;
        addr = ptr + offset;
    }
    Some(addr)
}

fn read_chain_f32(process: &Process, start: usize, offsets: &[usize]) -> Option<f32> {
    read_f32(process, resolve_chain(process, start, offsets)?)
}

fn read_f32(process: &Process, addr: usize) -> Option<f32> {
    read_memory_ex::<f32>(process, addr)
}

fn write_f32(process: &Process, addr: usize, value: f32) {
    write_memory_ex(process, addr, &value).expect("write failed");
}
