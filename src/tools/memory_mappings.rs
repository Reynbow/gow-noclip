pub const GOW_PROC_NAME: &str = "GoWR.exe";

pub const HEIGHT_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3E4]); //move left
pub const LEFT_RIGHT_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3E0]); // move right
pub const FORWARD_PTR: (usize, &[usize]) = (0x5D95520, &[0x2A0, 0x8C0]);
pub const DIRECTIONAL_PTR: (usize, &[usize]) = (0x5D954A0, &[0xB00, 0xDC0]); // tilts character left and right
// used for maintaining height
pub const ACCELERATION_PTR: (usize, &[usize]) = (0x29EBB00, &[0x3F4]);
