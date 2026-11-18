
pub fn i64_to_i32(v: i64) -> i32 {
    i32::try_from(v).expect("integer literal out of i32 range")
}

pub fn usize_to_u32(v: usize) -> u32 {
    u32::try_from(v).expect("value exceeds u32::MAX")
}

pub fn usize_to_u16(v: usize) -> u16 {
    u16::try_from(v).expect("value exceeds u16::MAX")
}

pub fn u16_to_u8(v: u16) -> u8 {
    u8::try_from(v).expect("value exceeds u8::MAX")
}

pub fn u16_high_byte(v: u16) -> u8 {
    u8::try_from(v >> 8).expect("u16 high byte")
}

pub fn u16_low_byte(v: u16) -> u8 {
    u8::try_from(v & 0xff).expect("u16 low byte")
}

pub fn i32_to_i8_byte(v: i32) -> u8 {
    i8::try_from(v)
        .expect("value out of i8 range")
        .to_ne_bytes()[0]
}

pub fn iconst_opcode(v: i32) -> u8 {
    u8::try_from(3u32 + u32::try_from(v).expect("iconst operand"))
        .expect("iconst opcode overflow")
}

pub fn branch_offset(branch_opcode_pc: usize, target_pc: usize) -> i16 {
    let from = i32::try_from(branch_opcode_pc).expect("branch pc");
    let to = i32::try_from(target_pc).expect("branch target");
    i16::try_from(to - from).expect("branch offset out of i16 range")
}
