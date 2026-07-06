pub const GOW_PROC_NAME: &str = "GoWR.exe";

/// Static global RVAs (still valid on current build for transform + direction).
pub const TRANSFORM_GLOBAL: usize = 0x5D95520;
pub const DIRECTIONAL_GLOBAL: usize = 0x5D954A0;

/// Pointer chain from transform global to the live character transform object.
pub const TRANSFORM_CHAIN: &[usize] = &[0x2A0];

/// Field offsets inside the transform object (updated game build).
pub const POS_X_OFFSET: usize = 0x8C8;
pub const POS_Y_OFFSET: usize = 0x8C4;
pub const POS_Z_OFFSET: usize = 0x8C0;
pub const ACCEL_OFFSET: usize = 0x8E0;

pub const DIRECTIONAL_CHAIN: &[usize] = &[0xB00, 0xDC0];

/// Legacy values kept for scan diagnostics only.
pub const LEGACY_POSITION_GLOBAL: usize = 0x29EBB00;
pub const LEGACY_POS_X_OFFSET: usize = 0x3E0;
pub const LEGACY_POS_Y_OFFSET: usize = 0x3E4;
pub const LEGACY_ACCEL_OFFSET: usize = 0x3F4;
