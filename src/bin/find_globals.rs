use std::collections::BTreeSet;
use std::fs;

fn main() {
    let path = r"C:\Program Files (x86)\Steam\steamapps\common\God of War Ragnarok\GoWR.exe";
    let data = fs::read(path).unwrap();
    let sections = parse_sections(&data);

    for field in [0x8B8u32, 0x8BC, 0x8C0, 0x8C4, 0x8C8, 0x8CC, 0x8D0, 0x8D4, 0x8D8, 0x8DC, 0x8E0, 0x8E4, 0x8E8] {
        let mut globals = BTreeSet::new();
        let b = field.to_le_bytes();
        for (name, va, vsize, raw) in &sections {
            if name != ".text" && !name.starts_with(".UBX") {
                continue;
            }
            let sec = &data[*raw..*raw + *vsize];
            for i in 0..sec.len().saturating_sub(8) {
                if sec[i] == 0xF3 && sec[i + 1] == 0x0F && (sec[i + 2] == 0x10 || sec[i + 2] == 0x11) {
                    if sec[i + 4..i + 8] == b {
                        collect_globals(sec, *va, i, &mut globals);
                    }
                }
            }
        }
        print!("+0x{field:X} ({})", globals.len());
        for g in globals {
            print!(" 0x{g:X}");
        }
        println!();
    }
}

fn parse_sections(data: &[u8]) -> Vec<(String, u64, usize, usize)> {
    let e_lfanew = u32::from_le_bytes(data[0x3C..0x40].try_into().unwrap()) as usize;
    let pe = e_lfanew + 4;
    let num_sections = u16::from_le_bytes(data[pe + 2..pe + 4].try_into().unwrap()) as usize;
    let opt_size = u16::from_le_bytes(data[pe + 16..pe + 18].try_into().unwrap()) as usize;
    let sec_table = pe + 20 + opt_size;
    let mut sections = Vec::new();
    for i in 0..num_sections {
        let sh = sec_table + i * 40;
        let va = u32::from_le_bytes(data[sh + 12..sh + 16].try_into().unwrap()) as u64;
        let vsize = u32::from_le_bytes(data[sh + 8..sh + 12].try_into().unwrap()) as usize;
        let raw = u32::from_le_bytes(data[sh + 20..sh + 24].try_into().unwrap()) as usize;
        let name = String::from_utf8_lossy(&data[sh..sh + 8])
            .trim_end_matches('\0')
            .to_string();
        if raw + vsize <= data.len() {
            sections.push((name, va, vsize, raw));
        }
    }
    sections
}

fn collect_globals(sec: &[u8], va: u64, i: usize, globals: &mut BTreeSet<u64>) {
    for back in 3..0x120 {
        if i < back {
            continue;
        }
        let j = i - back;
        if sec[j] == 0x48 && sec[j + 1] == 0x8B && (sec[j + 2] & 0xC7) == 0x05 {
            let disp = i32::from_le_bytes(sec[j + 3..j + 7].try_into().unwrap());
            let instr_rva = va + j as u64;
            let global_rva = (instr_rva as i64 + 7 + disp as i64) as u64;
            globals.insert(global_rva);
        }
    }
}
