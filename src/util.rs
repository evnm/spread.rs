use std::iter::range_step_inclusive;

/// Used to determine endianness.
static ENDIANTYPE: u32 = 0x80000080;

/// Convert an i32 to a 4-element byte vector.
pub fn int_to_bytes(i: u32) -> Vec<u8> {
    let mut vec: Vec<u8> = Vec::new();
    for p in range_step_inclusive(0u8, 24, 8) {
        vec.push(((i >> p) & 0xFF) as u8);
    }
    vec.reverse();
    vec
}

/// Convert a 4-element byte vector to an i32.
pub fn bytes_to_int(bytes: &[u8]) -> u32 {
    let i0 = bytes[0] as u32 & 0xFF;
    let i1 = bytes[1] as u32 & 0xFF;
    let i2 = bytes[2] as u32 & 0xFF;
    let i3 = bytes[3] as u32 & 0xFF;

    ((i0 << 24) | (i1 << 16) | (i2 << 8) | (i3))
}

/// Returns true if the argument int has the same endianness
/// as the local machine.
pub fn same_endianness(i: u32) -> bool {
    (i & ENDIANTYPE) == 0
}

/// Flips the endianness of the argument int.
pub fn flip_endianness(i: u32) -> u32 {
    let i0 = (i >> 24) & 0x000000ff;
    let i1 = (i >> 8) & 0x0000ff00;
    let i2 = (i << 8) & 0x00ff0000;
    let i3 = (i << 24) & 0xff000000;

    i0 | i1 | i2 | i3
}
