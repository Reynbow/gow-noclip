pub const GOW_PROC_NAME: &str = "GoWR.exe";

pub const HEIGHT_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3E4]);
pub const LEFT_RIGHT_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3E0]);
pub const FORWARD_PTR: (usize, &[usize]) = (0x5D953A8, &[0x3E8]);
pub const DIRECTIONAL_PTR: (usize, &[usize]) = (0x5D954A0, &[0xB00, 0xDC0]); // not using right now
// used for maintaining height
pub const ACCELERATION_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3F4]);
