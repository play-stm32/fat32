use block_device::BlockDevice;
use core::fmt::{Debug, Formatter, Result};
use crate::bpb::BIOSParameterBlock;
use crate::BUFFER_SIZE;
use crate::tool::{is_fat32, read_le_u16, read_le_u32};
use crate::dir::Dir;
use core::str;

#[derive(Copy, Clone)]
pub struct Volume<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug
{
    device: T,
    bpb: BIOSParameterBlock,
}

impl<T> Volume<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    /// get volume
    pub fn new(device: T) -> Volume<T> {
        let mut buf = [0; BUFFER_SIZE];
        device.read(&mut buf, 0, 1).unwrap();

        let mut volume_label = [0; 11];
        volume_label.copy_from_slice(&buf[0x47..0x52]);

        let mut file_system = [0; 8];
        file_system.copy_from_slice(&buf[0x52..0x5A]);

        if !is_fat32(&file_system) { panic!("not fat32 file_system"); }

        let bps = read_le_u16(&buf[0x0B..0x0D]);
        if bps as usize != BUFFER_SIZE {
            panic!("BUFFER_SIZE is {} Bytes, byte_per_sector is {} Bytes, no equal, \
            please edit feature {}", BUFFER_SIZE, bps, bps);
        }

        Volume::<T> {
            device,
            bpb: BIOSParameterBlock {
                byte_per_sector: bps,
                sector_per_cluster: buf[0x0D],
                reserved_sector: read_le_u16(&buf[0x0D..0x0F]),
                num_fat: buf[0x10],
                total_sector: read_le_u32(&buf[0x20..0x24]),
                sector_per_fat: read_le_u32(&buf[0x24..0x28]),
                root_cluster: read_le_u32(&buf[0x2C..0x30]),
                id: read_le_u32(&buf[0x43..0x47]),
                volume_label,
                file_system,
            },
        }
    }

    pub fn volume_label(&self) -> &str {
        str::from_utf8(&self.bpb.volume_label).unwrap()
    }

    /// into root_dir
    pub fn root_dir(&self) -> Dir<T> {
        Dir::<T> {
            device: self.device,
            bpb: self.bpb,
            dir_name: [0; 11],
            create_ms: 0,
            create_time: [0; 2],
            create_date: [0; 2],
            visit_date: [0; 2],
            edit_time: [0; 2],
            edit_date: [0; 2],
            dir_cluster: self.bpb.root_cluster,
            length: 0,
        }
    }
}

impl<T> Debug for Volume<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Volume")
            .field("byte_per_sector", &self.bpb.byte_per_sector)
            .field("sector_per_cluster", &self.bpb.sector_per_cluster)
            .field("reserved_sector", &self.bpb.reserved_sector)
            .field("num_fat", &self.bpb.num_fat)
            .field("total_sector", &self.bpb.total_sector)
            .field("sector_per_fat", &self.bpb.sector_per_fat)
            .field("root_cluster", &self.bpb.root_cluster)
            .field("id", &self.bpb.id)
            .field("volume_label", &self.volume_label())
            .field("file_system", &"FAT32")
            .finish()
    }
}