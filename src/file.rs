use block_device::BlockDevice;
use crate::bpb::BIOSParameterBlock;
use crate::directory_item::DirectoryItem;
use crate::fat::FAT;
use crate::BUFFER_SIZE;

#[derive(Debug)]
pub enum FileError {
    BufTooSmall,
}

#[derive(Debug, Copy, Clone)]
pub struct File<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub(crate) device: T,
    pub(crate) bpb: &'a BIOSParameterBlock,
    pub(crate) dir_cluster: u32,
    pub(crate) detail: DirectoryItem,
    pub(crate) fat: FAT<T>,
}

impl<'a, T> File<'a, T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        let length = self.detail.length().unwrap();
        let spc = self.bpb.sector_per_cluster_usize();
        let cluster_size = spc * BUFFER_SIZE;
        let mut number_of_blocks = spc;

        if buf.len() < length { return Err(FileError::BufTooSmall); }

        let mut index = 0;
        self.fat.map(|f| {
            let offset = self.bpb.offset(f.current_cluster);
            let end = if (buf.len() - index) < cluster_size {
                number_of_blocks = 1;
                index + (buf.len() % cluster_size)
            } else {
                index + cluster_size
            };
            self.device.read(&mut buf[index..end],
                             offset,
                             number_of_blocks).unwrap();
            index += cluster_size;
        }).last();

        Ok(length)
    }
}