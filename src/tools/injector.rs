use crate::tools::memory_mappings;
use libmem::*;

pub fn resolve_pointer_chain(
    process: &Process,
    mut addr: usize,
    offsets: &[usize],
) -> Option<usize> {
    for (i, offset) in offsets.iter().enumerate() {
        let ptr = libmem::read_memory_ex::<usize>(process, addr)?;
        println!(
            "Step {}: Read 0x{:X} -> 0x{:X} + 0x{:X}",
            i, addr, ptr, offset
        );
        addr = ptr + offset;
    }
    Some(addr)
}

pub fn change_height(process: &Process, base: usize, height_change: f32) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        println!("Resolved Height address: 0x{:X}", addr);
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            let new_val = val + height_change;
            if libmem::write_memory_ex(process, addr, &new_val).is_some() {
                println!("Height updated to {}", new_val);
                return Some(new_val);
            } else {
                println!("Failed to write new height at 0x{:X}", addr);
            }
        } else {
            println!("Failed to read current height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn set_height(process: &Process, base: usize, value: f32) -> Option<()> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        if libmem::write_memory_ex(process, addr, &value).is_some() {
            println!("Height set to {}", value);
            return Some(());
        } else {
            println!("Failed to write height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn get_height(process: &Process, base: usize) -> Option<f32> {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::HEIGHT_PTR.0,
        memory_mappings::HEIGHT_PTR.1,
    ) {
        println!("Resolved Height address: 0x{:X}", addr);
        if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
            println!("Current height: {}", val);
            return Some(val);
        } else {
            println!("Failed to read current height at 0x{:X}", addr);
        }
    } else {
        println!("Failed to resolve height pointer chain.");
    }
    None
}

pub fn move_left_right(process: &Process, base: usize, lr_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::LEFT_RIGHT_PTR.0,
        memory_mappings::LEFT_RIGHT_PTR.1,
    ) {
        println!("Resolved Left/Right address: 0x{:X}", addr);
        read_write_memory(process, addr, lr_change);
    } else {
        println!("Failed to resolve left-right pointer");
    }
}

pub fn move_forward(process: &Process, base: usize, forward_change: f32) {
    if let Some(addr) = resolve_pointer_chain(
        process,
        base + memory_mappings::FORWARD_PTR.0,
        memory_mappings::FORWARD_PTR.1,
    ) {
        println!("Resolved Forward/Back address: 0x{:X}", addr);
        read_write_memory(process, addr, forward_change);
    } else {
        println!("Failed to resolve forward pointer");
    }
}

fn read_write_memory(process: &Process, addr: usize, change: f32) -> Option<()> {
    if let Some(val) = libmem::read_memory_ex::<f32>(process, addr) {
        println!("Current value at 0x{:X}: {}", addr, val);
        let new_val = val + change;
        match libmem::write_memory_ex(process, addr, &new_val) {
            Some(()) => {
                println!("Value updated to {}", new_val);
                Some(())
            }
            None => {
                println!("Failed to write value at 0x{:X}:", addr);
                None
            }
        }
    } else {
        println!("Failed to read value at 0x{:X}", addr);
        None
    }
}
