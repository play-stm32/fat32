use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;
use crate::file::File;

#[derive(Debug)]
pub enum DirError {
    NoMatch,
    NoMatchDir,
    NoMatchFile,
    IllegalName,
}

enum CreateType {
    Dir,
    File,
}

#[derive(Debug, Copy, Clone)]
pub struct Dir<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub base: BASE,
    pub bpb: BIOSParameterBlock,
    pub dir_name: [u8; 11],
    pub create_ms: u8,
    pub create_time: [u8; 2],
    pub create_date: [u8; 2],
    pub visit_date: [u8; 2],
    pub edit_time: [u8; 2],
    pub edit_date: [u8; 2],
    pub dir_cluster: u32,
    pub length: u32,
}

impl<BASE> Dir<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub fn into_dir(&self, dir: &str) -> Result<Dir<BASE>, DirError> {
        match self.exist(dir) {
            Ok(buf) => {
                return Ok(self.get_dir(&buf.0));
            }
            Err(_) => {
                Err(DirError::NoMatchDir)
            }
        }
    }

    pub fn file(&self, file: &str) -> Result<File<BASE>, DirError> {
        match self.exist(file) {
            Ok(buf) => {
                return Ok(self.get_file(&buf.0, buf.1));
            }
            Err(_) => {
                Err(DirError::NoMatchFile)
            }
        }
    }

    pub fn create_dir(&self, dir: &str, time: &str) -> Result<(), DirError> {
        self.create(dir, time, CreateType::Dir)?;
        Ok(())
    }

    pub fn create_file(&self, file: &str, time: &str) -> Result<(), DirError> {
        self.create(file, time, CreateType::File)?;
        Ok(())
    }

    fn create(&self, name: &str, time: &str, create_type: CreateType) -> Result<(), DirError> {
        let illegal_char = "\\/:*?\"<>|";
        for ch in illegal_char.chars() {
            if name.contains(ch) {
                return Err(DirError::IllegalName);
            }
        }

        let fat_addr = self.get_blank_fat();
        let place = self.get_blank_place();
        let fat = fat_addr % 512 / 4;
        let place_index = place % 512;
        let offset = place - place_index;

        if name.is_ascii() && !name.contains(' ') && name.len() <= 8 {
            let mut buf = [0; 512];
            let high = fat & 0xFF00 >> 16;
            let low = fat & 0x00FF;

            self.base.read(&mut buf, self.bpb.offset(self.dir_cluster) + offset as u32
                           , 1).unwrap();

            for ch in name.chars().enumerate() {
                buf[ch.0 + place_index] = ch.1.to_ascii_uppercase() as u8;
            }

            for blank in name.len()..11 {
                buf[blank + place_index] = 0x20;
            }

            match create_type {
                CreateType::Dir => {
                    buf[0x0B + place_index] = 0x10;
                }
                CreateType::File => {
                    buf[0x0B + place_index] = 0x20;
                }
            }

            buf[0x0C + place_index] = 0x18;
            buf[0x14 + place_index] = (high & 0x0F) as u8;
            buf[0x15 + place_index] = (high & 0xF0 >> 8) as u8;
            buf[0x1A + place_index] = (low & 0x0F) as u8;
            buf[0x1B + place_index] = (low & 0xF0 >> 8) as u8;

            self.edit_fat(fat as u32, 0x0FFFFFFF);
            self.base.write(&buf, self.bpb.offset(self.dir_cluster) + offset as u32
                            , 1).unwrap();
        }

        Ok(())
    }

    fn exist(&self, name: &str) -> Result<([u8; 32], u32), DirError> {
        let illegal_char = "\\/:*?\"<>|";
        for ch in illegal_char.chars() {
            if name.contains(ch) {
                return Err(DirError::IllegalName);
            }
        }

        let op = |buf: &[u8]| -> [u8; 32] {
            let mut temp = [0; 32];
            for i in 0..32 {
                temp[i] = buf[i];
            }
            temp
        };

        let get_slice_index = |name: &str, end: usize| -> usize {
            let mut len = 0;
            for ch in name.chars().enumerate() {
                if (0..end).contains(&ch.0) {
                    len += ch.1.len_utf8();
                }
            }
            len
        };

        let is_short_name = name.is_ascii() && !name.contains(' ') && name.len() <= 8;
        let mut buf = [0; 512];
        let mut offset_count = 0;
        let mut step_count = 0;
        let mut copy_name = name;
        let mut long_cmp_done = false;

        for i in (0..).step_by(32) {
            if i % 512 == 0 {
                self.base.read(&mut buf, self.bpb.offset(self.dir_cluster) + offset_count * 512
                               , 1).unwrap();
                offset_count += 1;
            }

            if step_count != 0 {
                step_count -= 1;
                continue;
            }

            let offset = i - (offset_count as usize - 1) * 512;
            if buf[0x00 + offset] == 0x00 { break; }
            if long_cmp_done { return Ok((op(&buf[offset..offset + 32]), i as u32)); }

            if buf[0x00 + offset] == 0xE5 { continue; }
            if buf[0x0B + offset] == 0x0F {
                if is_short_name {
                    step_count = buf[0x00 + offset] & 0x1F;
                    continue;
                } else {
                    let len = copy_name.chars().count();
                    let count = buf[0x00 + offset] & 0x1F;
                    let info = self.get_long_name(&buf[offset..offset + 32]);
                    let part_name = core::str::from_utf8(&info.0[0..info.1]).unwrap();
                    let start_at = if len <= 13 { 0 } else { get_slice_index(copy_name, 13) };

                    if !&copy_name[start_at..].eq(part_name) {
                        copy_name = name;
                        step_count = count;
                        continue;
                    } else if start_at == 0 && count == 1 {
                        long_cmp_done = true;
                    } else {
                        copy_name = &copy_name[0..start_at];
                    }
                }
            } else {
                if is_short_name {
                    let info = self.get_short_name(&buf[offset..offset + 32]);
                    let file_name = core::str::from_utf8(&info.0[0..info.1]).unwrap();
                    if name.eq_ignore_ascii_case(file_name) {
                        return Ok((op(&buf[offset..offset + 32]), i as u32));
                    }
                }
            }
        }

        Err(DirError::NoMatch)
    }

    fn edit_fat(&self, loc: u32, value: u32) {
        let fat_addr = self.bpb.fat1();
        let offset = loc * 4;
        let offset_count = offset / 512;
        let offset = (offset % 512) as usize;

        let mut buf = [0; 512];
        self.base.read(&mut buf, fat_addr + offset_count * 512, 1).unwrap();

        buf[offset] = (value & 0xFF) as u8;
        buf[offset + 1] = ((value & 0xFF00) >> 8) as u8;
        buf[offset + 2] = ((value & 0xFF0000) >> 16) as u8;
        buf[offset + 3] = ((value & 0xFF00000) >> 24) as u8;

        self.base.write(&buf, fat_addr + offset_count * 512, 1).unwrap();
    }

    fn get_blank_place(&self) -> usize {
        let addr = self.bpb.offset(self.dir_cluster);
        let mut offset = 0;
        for _ in 0.. {
            let mut done = false;
            let mut buf = [0; 512];
            self.base.read(&mut buf, addr + offset as u32, 1).unwrap();
            for i in (0..512).step_by(32) {
                if buf[i] == 0x00 {
                    offset += i;
                    done = true;
                    break;
                }
            }
            if done { break; } else { offset += 512; }
        }
        offset
    }

    fn get_blank_fat(&self) -> usize {
        let fat_addr = self.bpb.fat1();
        let mut offset = 0;
        for _ in 0.. {
            let mut done = false;
            let mut buf = [0; 512];
            self.base.read(&mut buf, fat_addr + offset as u32, 1).unwrap();
            for i in (0..512).step_by(4) {
                if (buf[i] | buf[i + 1] | buf[i + 2] | buf[i + 3]) == 0 {
                    offset += i;
                    done = true;
                    break;
                }
            }
            if done { break; } else { offset += 512; }
        }
        offset
    }

    fn get_dir(&self, buf: &[u8]) -> Dir<BASE> {
        let mut dir_name = [0; 11];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];

        for i in 0x00..0x0B {
            dir_name[i] = buf[i];
        }

        Dir::<BASE> {
            base: self.base,
            bpb: self.bpb,
            dir_name,
            create_ms: buf[0x0D],
            create_time,
            create_date,
            visit_date: last_visit_date,
            edit_time,
            edit_date,
            dir_cluster: ((buf[0x15] as u32) << 24)
                | ((buf[0x14] as u32) << 16)
                | ((buf[0x1B] as u32) << 8)
                | (buf[0x1A] as u32),
            length: ((buf[0x1F] as u32) << 24)
                | ((buf[0x1E] as u32) << 16)
                | ((buf[0x1D] as u32) << 8)
                | (buf[0x1C] as u32),
        }
    }

    fn get_file(&self, buf: &[u8], offset: u32) -> File<BASE> {
        let mut file_name = [0; 8];
        let mut extension_name = [0; 3];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];

        let mut index = 0;
        for i in 0x00..0x08 {
            if buf[i] != 0x20 {
                file_name[index] = buf[i];
                index += 1;
            } else {
                break;
            }
        }

        index = 0;
        for i in 0x08..0x0B {
            if buf[i] != 0x20 {
                extension_name[index] = buf[i];
                index += 1;
            } else {
                break;
            }
        }

        let mut file_cluster = ((buf[0x15] as u32) << 24)
            | ((buf[0x14] as u32) << 16)
            | ((buf[0x1B] as u32) << 8)
            | (buf[0x1A] as u32);

        if file_cluster == 0 {
            let fat_addr = self.get_blank_fat();
            file_cluster = (fat_addr % 512 / 4) as u32;
            self.edit_fat(file_cluster, 0x0FFFFFFF);
        }

        File::<BASE> {
            base: self.base,
            bpb: self.bpb,
            dir_cluster: self.dir_cluster,
            offset,
            file_name,
            extension_name,
            create_ms: buf[0x0D],
            create_time,
            create_date,
            visit_date: last_visit_date,
            edit_time,
            edit_date,
            file_cluster,
            length: ((buf[0x1F] as u32) << 24)
                | ((buf[0x1E] as u32) << 16)
                | ((buf[0x1D] as u32) << 8)
                | (buf[0x1C] as u32),
        }
    }

    fn get_short_name(&self, buf: &[u8]) -> ([u8; 13], usize) {
        let mut file_name = [0; 13];
        let mut index = 0;

        for i in 0x00..=0x0A {
            if buf[i] != 0x20 {
                if i == 0x08 {
                    file_name[index] = '.' as u8;
                    index += 1;
                }
                file_name[index] = buf[i];
                index += 1;
            }
        }

        (file_name, index)
    }

    fn get_long_name(&self, buf: &[u8]) -> ([u8; 13 * 3], usize) {
        let mut res = ([0; 13 * 3], 0);

        let op = |res: &mut ([u8; 13 * 3], usize), start: usize, end: usize| {
            for i in (start..end).step_by(2) {
                if buf[i] == 0x00 && buf[i + 1] == 0x00 {
                    break;
                }

                let unicode = (((buf[i + 1] as u16) << 8) as u16) | buf[i] as u16;

                if unicode <= 0x007F {
                    res.0[res.1] = unicode as u8;
                    res.1 += 1;
                } else if unicode >= 0x0080 && unicode <= 0x07FF {
                    let part1 = (0b11000000 | (0b00011111 & (unicode >> 6))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & unicode) as u8;

                    res.0[res.1] = part1;
                    res.0[res.1 + 1] = part2;
                    res.1 += 2;
                } else if unicode >= 0x0800 {
                    let part1 = (0b11100000 | (0b00011111 & (unicode >> 12))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & (unicode >> 6)) as u8;
                    let part3 = (0b10000000 | (0b00111111) & unicode) as u8;

                    res.0[res.1] = part1;
                    res.0[res.1 + 1] = part2;
                    res.0[res.1 + 2] = part3;
                    res.1 += 3;
                }
            }
        };

        if buf[0x01] != 0xFF {
            op(&mut res, 0x01, 0x0A);
        }

        if buf[0x0E] != 0xFF {
            op(&mut res, 0x0E, 0x19);
        }

        if buf[0x1C] != 0xFF {
            op(&mut res, 0x1C, 0x1F);
        }

        return res;
    }
}