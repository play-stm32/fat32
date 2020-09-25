use core::convert::TryInto;
use core::str;

pub fn is_fat32(value: &[u8]) -> bool {
    let file_system_str = str::from_utf8(&value[0..5]).unwrap();
    file_system_str.eq("FAT32")
}

pub fn read_le_u16(input: &[u8]) -> u16 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u16>());
    u16::from_le_bytes(int_bytes.try_into().unwrap())
}

pub fn read_le_u32(input: &[u8]) -> u32 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u32>());
    u32::from_le_bytes(int_bytes.try_into().unwrap())
}