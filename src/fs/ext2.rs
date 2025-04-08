use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;

use crate::drivers::disk::Disk;
use crate::{info, math};

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Ext2SuperBlock {
    pub inode_count: u32,
    pub block_count: u32,
    pub reserved_block_superuser: u32,
    pub unallocated_block_count: u32,
    pub unallocated_inode_count: u32,
    pub superblock_container_block: u32, // also the starting block
    pub block_size: u32,                 // log2 (shift 1024 to the left)
    pub fragment_size: u32,              // log2
    pub block_group_count: u32,          // number of block in a group
    pub fragment_group_count: u32,       // number of fragment in a group
    pub inode_group_count: u32,          // number of inode in a group
    pub last_mount_time: u32,            // in POSIX time
    pub last_written_time: u32,          // in POSIX time
    pub mount_count_since_check: u16,
    pub mount_count_allowed_before_check: u16,
    pub magic: u16,
    pub fs_stat: u16,
    pub err_handler: u16, // tell to what to do in case of a error
    pub minor_version: u16,
    pub last_consistency_time_check: u32,
    pub interval_between_check: u32,
    pub system_id_creator: u32,
    pub major_version: u32,
    pub user_id: u16,
    pub group_id: u16,
}

impl Ext2SuperBlock {
    pub fn from_disk(disk: &mut impl Disk) -> (Option<Self>, Option<Ext2ExtendedSuperBlock>) {
        let mut vec = Vec::new();
        let bytes0 = disk.read_sector(2);
        let bytes1 = disk.read_sector(3);
        vec.extend_from_slice(&bytes0);
        vec.extend_from_slice(&bytes1);

        let buffer = &vec[..core::mem::size_of::<Self>()];
        let super_block = unsafe {
            (buffer.as_ptr() as *const Ext2SuperBlock)
                .as_ref()
                .expect("Failed to cast to a ref")
        };

        if !super_block.is_valid() {
            return (None, None);
        }
        let mut super_block_ext = None;
        if super_block.major_version >= 1 {
            super_block_ext = Ext2ExtendedSuperBlock::from_buffer(
                &vec[core::mem::size_of::<Self>()..core::mem::size_of::<Ext2ExtendedSuperBlock>()],
            )
        }

        (Some(*super_block), super_block_ext)
    }

    pub fn is_valid(&self) -> bool {
        self.magic == 0xef53
    }

    pub fn get_block_group_count(&self) -> i64 {
        math::ceil(self.block_count as f64 / self.block_group_count as f64)
    }

    pub fn block_size(&self) -> u32 {
        1024 << self.block_size
    }

    pub fn get_sector_for_block(&self, blockid: u32) -> core::ops::Range<u32> {
        let sector_n = self.block_size() / 512;

        let i = sector_n * blockid;
        i..i + sector_n
    }

    pub fn read_block(&self, blockid: u32, disk: &mut impl Disk) -> Vec<u8> {
        let sectors = self.get_sector_for_block(blockid);

        let mut buffer = Vec::new();

        for sector in sectors {
            let v = disk.read_sector(sector as u64);
            buffer.extend_from_slice(&v);
        }

        buffer
    }

    pub fn write_block(
        &self,
        block_id: u32,
        disk: &mut impl Disk,
        buf: &[u8],
    ) -> Result<(), String> {
        let block_size = self.block_size();
        let mut buf = buf.to_vec();
        if (block_size as usize) > buf.len() {
            buf.resize(block_size as usize, 0);
        }

        let sectors = self.get_sector_for_block(block_id);
        let mut off = 0;
        for sector in sectors {
            disk.write_sector(sector as u64, &buf[off..off + 512]);
            off += 512;
        }
        Ok(())
    }

    pub fn flush(&mut self, disk: &mut impl Disk) {
        let s = unsafe {
            core::slice::from_raw_parts(
                self as *const Ext2SuperBlock as *const u8,
                core::mem::size_of::<Self>(),
            )
        };

        let mut v = Vec::new();
        v.extend_from_slice(&s);
        v.resize(512, 0);

        disk.write_sector(2,&v);
    }
}

pub const FS_STATE_CLEAN: u32 = 1;
pub const FS_STATE_ERROR: u32 = 2;

pub const ERR_HANDLER_IGNORE: u16 = 1;
pub const ERR_HANDLER_REMOUNT_READONLY: u16 = 2;
pub const ERR_HANDLER_KERNEL_PANIC: u16 = 3;

pub const SYSTEM_ID_LINUX: u32 = 0;
pub const SYSTEM_ID_GNU_HURD: u32 = 1;
pub const SYSTEM_ID_MASIX: u32 = 2;
pub const SYSTEM_ID_FREEBSD: u32 = 3;
pub const SYSTEM_ID_LITES: u32 = 4;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Ext2ExtendedSuperBlock {
    pub first_no_reserved_inode: u32,
    pub size_inode: u16,
    pub block_group: u16,
    pub optional_feature: u32,
    pub required_feature: u32,
    pub no_supported_feature: u32,
    pub fs_id: [u8; 16],    // C string
    pub vol_name: [u8; 64], // C string
    pub compression_algo: u32,
    pub preallocated_block_for_file: u8,
    pub preallocated_block_for_dir: u8,
    pub _unused: u16,
    pub journal_id: [u8; 16],
    pub journal_inode: u32,
    pub journal_device: u32,
    pub head_orphan_inode_list: u32,
}

impl Ext2ExtendedSuperBlock {
    pub fn from_buffer(buf: &[u8]) -> Option<Ext2ExtendedSuperBlock> {
        let ext2_entended_superblock =
            unsafe { (buf.as_ptr() as *const Ext2ExtendedSuperBlock).as_ref()? };

        Some(*ext2_entended_superblock)
    }

    pub fn optional_feature(&self) -> Ext2OptionalFeature {
        Ext2OptionalFeature::from(self.optional_feature)
    }

    pub fn required_feature(&self) -> Ext2RequiredFeature {
        Ext2RequiredFeature::from(self.required_feature)
    }

    pub fn read_only_feature(&self) -> Ext2ReadOnlyFeature {
        Ext2ReadOnlyFeature::from(self.no_supported_feature)
    }

    pub fn flush(&self, disk: &mut impl Disk) {
        let x = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        };
        let mut v = Vec::with_capacity(512);
        v.extend_from_slice(&x);
        disk.write_sector(3, &v);
    }
}

pub struct Ext2OptionalFeature {
    pub preallocated_block: bool,
    pub afs_server_inode: bool,
    pub journal: bool,
    pub extended_attribute: bool,
    pub can_resize_itself: bool,
    pub hash_index: bool,
}

impl From<u32> for Ext2OptionalFeature {
    fn from(flags: u32) -> Self {
        Ext2OptionalFeature {
            preallocated_block: flags & OPTIONAL_FEATURE_FLAG_PREALLOCATED != 0,
            afs_server_inode: flags & OPTIONAL_FEATURE_FLAG_AFS_SERVER_INODE != 0,
            journal: flags & OPTIONAL_FEATURE_FLAG_JOURNAL != 0,
            extended_attribute: flags & OPTIONAL_FEATURE_FLAG_EXTENDED_ATTR != 0,
            can_resize_itself: flags & OPTIONAL_FEATURE_FLAG_CAN_RESIZE_ITSELF != 0,
            hash_index: flags & OPTIONAL_FEATURE_FLAG_HASH_INDEX != 0,
        }
    }
}

pub struct Ext2RequiredFeature {
    pub compression_used: bool,
    pub type_field: bool,
    pub replay_journal: bool,
    pub journal_device: bool,
}

impl From<u32> for Ext2RequiredFeature {
    fn from(flags: u32) -> Self {
        Ext2RequiredFeature {
            compression_used: flags & REQUIRED_FEATURE_FLAG_COMPRESION_USED != 0,
            type_field: flags & REQUIRED_FEATURE_FLAG_TYPE_FIELD != 0,
            replay_journal: flags & REQUIRED_FEATURE_FLAG_REPLAY_JOURNAL != 0,
            journal_device: flags & REQUIRED_FEATURE_FLAG_JOURNAL_DEVICE != 0,
        }
    }
}

pub struct Ext2ReadOnlyFeature {
    pub sparse_userblock: bool,
    pub sixty_four_bit_filename: bool,
    pub binary_tree_dir: bool,
}

impl From<u32> for Ext2ReadOnlyFeature {
    fn from(flags: u32) -> Self {
        Ext2ReadOnlyFeature {
            sparse_userblock: flags & READ_ONLY_FEATURE_FLAG_SPARCE_USERBLOCK != 0,
            sixty_four_bit_filename: flags & READ_ONLY_FEATURE_FLAG_64BIT_FILENAME != 0,
            binary_tree_dir: flags & READ_ONLY_FEATURE_FLAG_BINARY_TREE_DIR != 0,
        }
    }
}

pub const OPTIONAL_FEATURE_FLAG_PREALLOCATED: u32 = 0x1;
pub const OPTIONAL_FEATURE_FLAG_AFS_SERVER_INODE: u32 = 0x2;
pub const OPTIONAL_FEATURE_FLAG_JOURNAL: u32 = 0x4;
pub const OPTIONAL_FEATURE_FLAG_EXTENDED_ATTR: u32 = 0x8;
pub const OPTIONAL_FEATURE_FLAG_CAN_RESIZE_ITSELF: u32 = 0x10;
pub const OPTIONAL_FEATURE_FLAG_HASH_INDEX: u32 = 0x20;

pub const REQUIRED_FEATURE_FLAG_COMPRESION_USED: u32 = 0x1;
pub const REQUIRED_FEATURE_FLAG_TYPE_FIELD: u32 = 0x2;
pub const REQUIRED_FEATURE_FLAG_REPLAY_JOURNAL: u32 = 0x4;
pub const REQUIRED_FEATURE_FLAG_JOURNAL_DEVICE: u32 = 0x8;

pub const READ_ONLY_FEATURE_FLAG_SPARCE_USERBLOCK: u32 = 0x1;
pub const READ_ONLY_FEATURE_FLAG_64BIT_FILENAME: u32 = 0x2;
pub const READ_ONLY_FEATURE_FLAG_BINARY_TREE_DIR: u32 = 0x4;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ext2BlockGroupDescriptor {
    pub block_bitmap_addr: u32,
    pub inode_bitmap_addr: u32,
    pub inode_table_addr: u32,
    pub unallocated_block_count: u16,
    pub unallocated_inode_count: u16,
    pub dir_count: u16,
    pub _unused: [u8; 14],
}

impl Ext2BlockGroupDescriptor {
    pub fn get_group_from_inode_addr(inode: u32, superblock: Ext2SuperBlock) -> u32 {
        math::ceil((inode - 1) as f64 / superblock.inode_group_count as f64) as u32
    }

    pub fn get_inode_index_from_addr(inode_addr: u32, superblock: Ext2SuperBlock) -> u32 {
        (inode_addr - 1) % superblock.inode_group_count
    }

    pub fn get_block_from_inode_index(
        inode_index: u32,
        superblock: Ext2SuperBlock,
        superblock_ext: Option<Ext2ExtendedSuperBlock>,
    ) -> u32 {
        (inode_index * superblock_ext.map(|x| x.size_inode as u32).unwrap_or(128))
            / superblock.block_size()
    }

    pub fn from_disk(superblock: Ext2SuperBlock, disk: &mut impl Disk) -> Option<Vec<Self>> {
        let group_count = superblock.get_block_group_count();
        let block_size = superblock.block_size();
        let mut v = Vec::new();
        let mut b = if block_size == 1024 { 2 } else { 1 };
        for i in 0..group_count {
            let buffer = superblock.read_block(b, disk);
            let bgd = unsafe { (buffer.as_ptr() as *const Ext2BlockGroupDescriptor).as_ref()? };
            v.push(*bgd);
            b += 1;
        }

        Some(v)
    }

    pub fn flush(&self, superblock: Ext2SuperBlock, disk: &mut impl Disk) -> Result<(), String> {
        let x = unsafe {
            core::slice::from_raw_parts(
                self as *const Ext2BlockGroupDescriptor as *const u8,
                core::mem::size_of::<Self>(),
            )
        };
        let mut v = Vec::with_capacity(512);
        v.extend_from_slice(&x);
        superblock.write_block(
            if superblock.block_size() == 1024 {
                2
            } else {
                1
            },
            disk,
            &v,
        )
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Inode {
    pub type_and_perm: u16,
    pub user_id: u16,
    pub size_low: u32,
    pub last_access_time: u32,
    pub creation_time: u32,
    pub last_modif_time: u32,
    pub del_time: u32,
    pub group_id: u16,
    pub hard_link_count: u16,
    pub disk_sector_count: u32,
    pub flags: u32,
    pub os_spec0: u32,
    pub dbp: [u32; 12], //  DBP = Direct Block Pointer
    pub sibp: u32,      //  SIBP = Singly Indirect Block Pointer
    pub dibp: u32,      //  DIBP = Doubly Indirect Block Pointer
    pub tibp: u32,      //  TIBP = Triply Indirect Block Pointer
    pub generation_number: u32,
    pub extended_attr_block: u32,
    pub size_high_or_acl: u32,
    pub block_addr: u32,
    pub os_spec1: [u8; 44],
}

impl core::fmt::Debug for Inode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Inode")
            .field("type_and_perm", &self.type_and_perm)
            .field("user_id", &self.user_id)
            .field("size_low", &self.size_low)
            .field("last_access_time", &self.last_access_time)
            .field("creation_time", &self.creation_time)
            .field("last_modif_time", &self.last_modif_time)
            .field("del_time", &self.del_time)
            .field("group_id", &self.group_id)
            .field("hard_link_count", &self.hard_link_count)
            .field("disk_sector_count", &self.disk_sector_count)
            .field("flags", &self.flags)
            .field("os_spec0", &self.os_spec0)
            .field("dbp", &self.dbp)
            .field("sibp", &self.sibp)
            .field("dibp", &self.dibp)
            .field("tibp", &self.tibp)
            .field("generation_number", &self.generation_number)
            .field("extended_attr_block", &self.extended_attr_block)
            .field("size_high_or_acl", &self.size_high_or_acl)
            .field("block_addr", &self.block_addr)
            .finish()
    }
}

pub const INODE_TYPE_FIFO: u16 = 0x1000;
pub const INODE_TYPE_CHAR: u16 = 0x2000;
pub const INODE_TYPE_DIR: u16 = 0x4000;
pub const INODE_TYPE_BLOCK: u16 = 0x6000;
pub const INODE_TYPE_REG: u16 = 0x8000;
pub const INODE_TYPE_SYMLINK: u16 = 0xa000;
pub const INODE_TYPE_UNIX_SOCK: u16 = 0xc000;

pub const INODE_PERM_OTHER_EXEC: u16 = 0x0001;
pub const INODE_PERM_OTHER_WRITE: u16 = 0x0002;
pub const INODE_PERM_OTHER_READ: u16 = 0x0004;
pub const INODE_PERM_GROUP_EXEC: u16 = 0x0008;
pub const INODE_PERM_GROUP_WRITE: u16 = 0x0010;
pub const INODE_PERM_GROUP_READ: u16 = 0x0020;
pub const INODE_PERM_USER_EXEC: u16 = 0x0040;
pub const INODE_PERM_USER_WRITE: u16 = 0x080;
pub const INODE_PERM_USER_READ: u16 = 0x0100;
pub const INODE_PERM_STICKY_BIT: u16 = 0x0200;
pub const INODE_PERM_SET_GROUP_ID: u16 = 0x0400;
pub const INODE_PERM_SET_USER_ID: u16 = 0x0800;

pub const INODE_FLAG_SECURE_DEL: u16 = 0x00001;
pub const INODE_FLAG_KEEP_COPY: u16 = 0x00002;
pub const INODE_FLAG_FILE_COMPRESSION: u16 = 0x00004;
pub const INODE_FLAG_SYNC_DATA: u16 = 0x00008;
pub const INODE_FLAG_IMMUT_FILE: u16 = 0x00010;
pub const INODE_FLAG_APPEND_ONLY: u16 = 0x00020;
pub const INODE_FLAG_FILE_NOT_INCLUDED_IN_DUMP: u32 = 0x00040;
pub const INODE_FLAG_LAST_ACCESS_NO_UPDATE: u32 = 0x00080;
pub const INODE_FLAG_HASH_INDEXED_DIR: u32 = 0x10000;
pub const INODE_FLAG_AFS_DIR: u32 = 0x20000;
pub const INODE_FLAG_JOURNAL_FILE_DATA: u32 = 0x40000;

impl Inode {
    pub fn new() -> Self {
        Inode {
            type_and_perm: 0,
            user_id: 0,
            size_low: 0,
            last_access_time: 0,
            creation_time: 0,
            last_modif_time: 0,
            del_time: 0,
            group_id: 0,
            hard_link_count: 0,
            disk_sector_count: 0,
            flags: 0,
            os_spec0: 0,
            dbp: [0; 12],
            sibp: 0,
            dibp: 0,
            tibp: 0,
            generation_number: 0,
            extended_attr_block: 0,
            size_high_or_acl: 0,
            block_addr: 0,
            os_spec1: [0; 44],
        }
    }
    pub fn from_buffer(buf: &[u8]) -> Option<Inode> {
        unsafe { (buf.as_ptr() as *const Inode).as_ref().cloned() }
    }
    pub fn type_(&self) -> u16 {
        ((self.type_and_perm & 0xF000) >> 12)
    }

    pub fn perm(&self) -> u16 {
        self.type_and_perm & 0x1FFF
    }

    pub fn is_dir(&self) -> bool {
        self.type_() & (INODE_TYPE_DIR >> 12) != 0
    }

    pub fn is_file(&self) -> bool {
        self.type_() & (INODE_TYPE_REG >> 12) != 0
    }

    pub fn flush(&self) -> Vec<u8> {
        let mut b = Vec::new();

        b.extend_from_slice(&self.type_and_perm.to_le_bytes());
        b.extend_from_slice(&self.user_id.to_le_bytes());
        b.extend_from_slice(&self.size_low.to_le_bytes());
        b.extend_from_slice(&self.last_access_time.to_le_bytes());
        b.extend_from_slice(&self.creation_time.to_le_bytes());
        b.extend_from_slice(&self.last_modif_time.to_le_bytes());
        b.extend_from_slice(&self.del_time.to_le_bytes());
        b.extend_from_slice(&self.group_id.to_le_bytes());
        b.extend_from_slice(&self.hard_link_count.to_le_bytes());
        b.extend_from_slice(&self.disk_sector_count.to_le_bytes());
        b.extend_from_slice(&self.flags.to_le_bytes());
        b.extend_from_slice(&self.os_spec0.to_le_bytes());
        for i in 0..12 {
            b.extend_from_slice(&self.dbp[i].to_le_bytes());
        }
        b.extend_from_slice(&self.sibp.to_le_bytes());
        b.extend_from_slice(&self.dibp.to_le_bytes());
        b.extend_from_slice(&self.tibp.to_le_bytes());
        b.extend_from_slice(&self.generation_number.to_le_bytes());
        b.extend_from_slice(&self.extended_attr_block.to_le_bytes());
        b.extend_from_slice(&self.size_high_or_acl.to_le_bytes());
        b.extend_from_slice(&self.block_addr.to_le_bytes());

        for i in 0..44 {
            b.push(self.os_spec1[i]);
        }

        b
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C, align(4))]
pub struct DirectoryEntry {
    pub inode: u32,
    pub entry_size: u16,
    pub name_length: u8,
    pub file_type: u8,
    pub name: Vec<u8>,
}

impl DirectoryEntry {
    pub fn from_buffer<'a>(buf: &mut impl Iterator<Item = &'a u8>) -> Option<Self> {
        let inode = u32::from_le_bytes(buf.copied().next_chunk::<4>().unwrap());
        let entry_size = u16::from_le_bytes(buf.copied().next_chunk::<2>().unwrap());
        let name_length = *buf.next().unwrap();
        let file_type = *buf.next().unwrap();

        let slice = buf.take(name_length as usize).copied().collect();

        Some(Self {
            inode,
            entry_size,
            name_length,
            file_type,
            name: slice,
        })
    }

    pub fn name(&self) -> Option<&str> {
        core::str::from_utf8(&self.name).ok()
    }

    pub fn file_type(&self) -> Option<DirTypeIndicator> {
        Some(match self.file_type {
            0 => DirTypeIndicator::Unknown,
            1 => DirTypeIndicator::RegFile,
            2 => DirTypeIndicator::Dir,
            3 => DirTypeIndicator::CharDevice,
            4 => DirTypeIndicator::BlockDevice,
            5 => DirTypeIndicator::FIFO,
            6 => DirTypeIndicator::Socket,
            7 => DirTypeIndicator::SymLink,
            _ => return None,
        })
    }

    pub fn flush(&self) -> Vec<u8> {
        let mut b = Vec::new();

        b.extend_from_slice(&self.inode.to_le_bytes());
        b.extend_from_slice(&self.entry_size.to_le_bytes());
        b.extend_from_slice(&self.name_length.to_le_bytes());
        b.extend_from_slice(&self.file_type.to_le_bytes());
        b.extend_from_slice(&self.name);

        b
    }
}

pub const TYPE_INDICATOR_UNKNWON: u8 = 0;
pub const TYPE_INDICATOR_REG_FILE: u8 = 1;
pub const TYPE_INDICATOR_DIR: u8 = 2;
pub const TYPE_INDICATOR_CHAR_DEVICE: u8 = 3;
pub const TYPE_INDICATOR_BLOCK_DEVICE: u8 = 4;
pub const TYPE_INDICATOR_FIFO: u8 = 5;
pub const TYPR_INDICATOR_SOCKET: u8 = 6;
pub const TYPE_INDICATOR_SYM_LINK: u8 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirTypeIndicator {
    Unknown = 0,
    RegFile = 1,
    Dir = 2,
    CharDevice = 3,
    BlockDevice = 4,
    FIFO = 5,
    Socket = 6,
    SymLink = 7,
}

pub struct Ext2FS<T: Disk + Clone> {
    pub super_block: Ext2SuperBlock,
    pub super_block_ext: Option<Ext2ExtendedSuperBlock>,
    pub bgd: Vec<Ext2BlockGroupDescriptor>,
    disk: T,
}

impl<T> Ext2FS<T>
where
    T: Disk + Clone,
{
    pub fn from_disk(disk: &mut T) -> Option<Self> {
        let (super_block, super_block_ext) = Ext2SuperBlock::from_disk(disk);

        let super_block = super_block?;

        let bgd = Ext2BlockGroupDescriptor::from_disk(super_block, disk)?;

        Some(Self {
            super_block,
            super_block_ext,
            bgd,
            disk: disk.clone(),
        })
    }

    pub fn read_inode(&mut self, inode_number: u32) -> Option<Inode> {
        let inode_size = self
            .super_block_ext
            .map(|e| e.size_inode as u32)
            .unwrap_or(128);
        let inodes_per_group = self.super_block.inode_group_count;
        let group = (inode_number - 1) / inodes_per_group;
        let index = (inode_number - 1) % inodes_per_group;
        let inode_table_block = self.bgd.get(group as usize)?.inode_table_addr;
        let offset = index * inode_size;

        let block_size = self.super_block.block_size();
        let block = inode_table_block + (offset / block_size);
        let block_offset = offset % block_size;

        let data = self.super_block.read_block(block, &mut self.disk);
        let inode_data = &data[block_offset as usize..(block_offset + inode_size) as usize];

        unsafe { (inode_data.as_ptr() as *const Inode).as_ref().cloned() }
    }

    pub fn write_inode(&mut self, inode_number: u32, inode: Inode) -> Result<(), super::Error> {
        let inode_size = self
            .super_block_ext
            .map(|e| e.size_inode as u32)
            .unwrap_or(128);
        let inodes_per_group = self.super_block.inode_group_count;
        let group = (inode_number - 1) / inodes_per_group;
        let index = (inode_number - 1) % inodes_per_group;
        let inode_table_block = self
            .bgd
            .get(group as usize)
            .ok_or(super::Error::FileNotFound)?
            .inode_table_addr;
        let offset = index * inode_size;

        let block_size = self.super_block.block_size();
        let block = inode_table_block + (offset / block_size);
        let block_offset = offset % block_size;

        let mut data = self.super_block.read_block(block, &mut self.disk);
        let mut inode = inode.flush();
        inode.resize(inode_size as usize, 0);
        data[block_offset as usize..(block_offset + inode_size) as usize].copy_from_slice(&inode);

        self.super_block
            .write_block(block, &mut self.disk, &data)
            .map_err(|x| super::Error::KernelError(x))
    }

    pub fn read_file(&mut self, inode: Inode) -> Result<Vec<u8>, super::Error> {
        let block_size = self.super_block.block_size();
        let filesize = inode.size_low; // in byte
        let mut remaining_size = filesize;
        let mut data = Vec::new();
        let mut current_dbp = 0;

        while remaining_size != 0 {
            let current_block = self.read_block(inode.dbp[current_dbp]);
            if remaining_size < block_size {
                data.extend_from_slice(&current_block[..remaining_size as usize]);
                break;
            } else {
                data.extend(current_block);
                remaining_size -= block_size;
            }
        }

        Ok(data)
    }

    pub fn read_directory(&mut self, inode: u32) -> Result<Vec<DirectoryEntry>, super::Error> {
        let inode_data = self.read_inode(inode).ok_or(super::Error::FileNotFound)?;
        let file = self.read_file(inode_data)?;

        let mut entries = Vec::new();
        let mut offset = 0;

        while offset < file.len() {
            let x = &mut file[offset..].iter();
            let entry = DirectoryEntry::from_buffer(x).unwrap();
            if entry.inode == 0 {
                break;
            }
            entries.push(entry.clone());
            offset += entry.entry_size as usize;
        }

        Ok(entries)
    }

    pub fn read_block(&mut self, block_id: u32) -> Vec<u8> {
        self.super_block.read_block(block_id, &mut self.disk)
    }

    pub fn read_inode_by_name(
        &mut self,
        dir_inode: u32,
        name: &str,
    ) -> Result<u32, super::Error> {
        let dirs = self.read_directory(dir_inode)?;
        for dir in dirs {
            if let Some(dname) = dir.name() {
                if dname == name {
                    return Ok(dir.inode);
                }
            }
        }
        Err(super::Error::FileNotFound)
    }

    pub fn read_path(&mut self, path: super::Path) -> Result<u32, super::Error> {
        if path.is_relative() {
            return Err(super::Error::CantUseRelPath);
        }

        let dirs = self.read_directory(2)?;
        let comps = path.components();

        let mut current_inode = 2;

        for (i, comp) in comps.iter().enumerate() {
            current_inode = self.read_inode_by_name(current_inode, comp)?;
            let inode = self.read_inode(current_inode).ok_or(super::Error::FileNotFound)?;

            if !inode.is_dir() && i != comps.len() - 1 {
                return Err(super::Error::NotADirectory);
            }
        }

        Ok(current_inode)
    }

    pub fn write_block(&mut self, block_id: u32, buf: &[u8]) {
        self.super_block
            .write_block(block_id, &mut self.disk, buf)
            .unwrap()
    }

    pub fn allocate_block(&mut self) -> Option<u32> {
        let block_grp = self.block_group_allocate()?;
        let bitmap_block = self.bgd[block_grp as usize].block_bitmap_addr;
        let block_size = self.super_block.block_size();
        let mut bitmap = self.super_block.read_block(bitmap_block, &mut self.disk);
    
        for byte_index in 0..bitmap.len() {
            let byte = bitmap[byte_index];
    
            for bit_index in 0..8 {
                if (byte & (1 << bit_index)) == 0 {
                    // Ce bloc est libre
                    bitmap[byte_index] |= 1 << bit_index;
    
                    // Sauvegarde la bitmap modifiée
                    self.write_block(bitmap_block, &bitmap);
                    self.bgd[block_grp as usize].unallocated_block_count -= 1;
    
                    // Calcul de l’index du bloc alloué
                    let block_index_within_group = (byte_index * 8 + bit_index) as u32;
                    let blocks_per_group = self.super_block.block_group_count;
    
                    // Adresse réelle du bloc (relatif au groupe)
                    let block_absolute = self.super_block.block_group_count * block_grp + block_index_within_group;
    
                    return Some(block_absolute);
                }
            }
        }
    
        None
    }
    

    pub fn block_group_allocate(&self) -> Option<u32> {
        for i in 0..self.bgd.len() {
            if self.bgd[i].unallocated_block_count > 0 {
                return Some(i as u32);
            }
        }
        None
    }

    pub fn allocate_inode(&mut self) -> Option<u32> {
        let group = self.block_group_allocate()?;
        let block_id = self.bgd[group as usize].inode_bitmap_addr;
        let block_size = self.super_block.block_size();
        let mut bitmap = self.super_block.read_block(block_id, &mut self.disk);
        for byte_index in 0..bitmap.len() {
            if byte_index == 0 { continue; } // skip inode 1-8 si tu veux les réserver
        
            let byte = bitmap[byte_index];
            for bit_index in 0..8 {
                if (byte & (1 << bit_index)) == 0 {
                    // Marque le bit comme utilisé
                    bitmap[byte_index] |= 1 << bit_index;
        
                    self.write_block(block_id, &bitmap);
                    self.bgd[group as usize].unallocated_inode_count -= 1;
        
                    let inode_index = byte_index * 8 + bit_index;
                    // inode 1 est en position 0
                    return Some((inode_index+1) as u32);
                }
            }
        }
        
        None
    }

    pub fn is_unallocated(&mut self, inode: u32) -> bool {
        // Les inodes commencent à 1, donc on ajuste l'index
        let inode_index = inode - 1;
    
        let inodes_per_group = self.super_block.inode_group_count;
        let group = inode_index / inodes_per_group;
        let index = inode_index % inodes_per_group;
    
        let block_id = self.bgd[group as usize].inode_bitmap_addr;
    
        let bitmap = self.super_block.read_block(block_id, &mut self.disk);
    
        let byte_index = (index / 8) as usize;
        let bit_offset = index % 8;
    
        if byte_index >= bitmap.len() {
            return true; // Considérer comme non alloué si out of bounds
        }
    
        // Si le bit est à 0 → inode non alloué
        (bitmap[byte_index] & (1 << bit_offset)) == 0
    }
    

    pub fn insert_inode(
        &mut self,
        parent_inode_number: u32,
        name: &str,
        inode: Inode,
    ) -> Result<(), super::Error> {
        let mut parent_inode = self.read_inode(parent_inode_number).ok_or(super::Error::FileNotFound)?;
        let mut dir = self.read_directory(parent_inode_number)?;
        let inode_id = self.allocate_inode().ok_or(super::Error::NotEnoughSpace)?;
        let mut new_entry = DirectoryEntry {
            inode: inode_id,
            entry_size: 0, // on mettra la vraie valeur après
            name_length: name.len() as u8,
            file_type: TYPE_INDICATOR_REG_FILE,
            name: name.as_bytes().to_vec(),
        };
    
        let block_size = self.super_block.block_size() as usize;
    
        // let mut data = Vec::new();
        let mut blocks = Vec::new();
        let mut current_block_entries: Vec<DirectoryEntry> = Vec::new();
        let mut current_block_size = 0;
    
        for entry in dir {
            let entry_data = entry.flush();
            let padding_len = entry.entry_size as usize - entry_data.len();
            let total_entry_len = entry_data.len() + padding_len;
    
            if current_block_size + total_entry_len > block_size {
                // Bloc plein : on flush
                let mut block = Vec::new();
                for e in &current_block_entries {
                    let data = e.flush();
                    let padding = e.entry_size as usize - data.len();
                    block.extend_from_slice(&data);
                    block.extend(core::iter::repeat(0).take(padding));
                }
                blocks.push(block);
                current_block_entries.clear();
                current_block_size = 0;
            }
    
            current_block_entries.push(entry);
            current_block_size += total_entry_len;
        }
    
        // On tente d’ajouter dans le dernier bloc
        let mut last_block_filled = false;
        if !current_block_entries.is_empty() {
            let last = current_block_entries.last_mut().unwrap();
            let last_entry_size = 8 + ((last.name_length as usize + 3) & !3);
            let space = last.entry_size as usize - last_entry_size;
            let new_entry_size = 8 + ((name.len() + 3) & !3);
    
            if space >= new_entry_size {
                // On split la dernière entrée
                last.entry_size = last_entry_size as u16;
    
                new_entry.entry_size = (block_size - current_block_size + space) as u16;
                current_block_entries.push(new_entry.clone());
                last_block_filled = true;
            }
        }
    
        if !last_block_filled {
            // Nouveau bloc
            new_entry.entry_size = block_size as u16;
            blocks.push({
                let mut block = new_entry.flush();
                block.resize(block_size, 0);
                block
            });
        } else {
            // Flush du dernier bloc avec la nouvelle entrée
            let mut block = Vec::new();
            for e in &current_block_entries {
                let data = e.flush();
                let padding = e.entry_size as usize - data.len();
                block.extend_from_slice(&data);
                block.extend(core::iter::repeat(0).take(padding));
            }
            blocks.push(block);
        }
    
        // Écriture des blocs
        for (i, block) in blocks.iter().enumerate() {
            let block_num = if i < parent_inode.dbp.len() && parent_inode.dbp[i] != 0 {
                parent_inode.dbp[i]
            } else {
                let b = self.allocate_block().ok_or(super::Error::NotEnoughSpace)?;
                parent_inode.dbp[i] = b;
                b
            };
            self.write_block(block_num, block);
        }
    
        self.write_inode(parent_inode_number, parent_inode)?;
        self.write_inode(inode_id, inode)?;
        self.flush();
    
        Ok(())
    }

    pub fn insert_inode2(
        &mut self,
        parent_inode_number: u32,
        name: &str,
        inode: Inode,
    ) -> Result<(), super::Error> {
        let mut parent_inode = self.read_inode(parent_inode_number).ok_or(super::Error::FileNotFound)?;
        let inode_id = self.allocate_inode().ok_or(super::Error::NotEnoughSpace)?;
        let mut new_entry = DirectoryEntry {
            inode: inode_id,
            entry_size: 0, 
            name_length: name.len() as u8,
            file_type: TYPE_INDICATOR_REG_FILE,
            name: name.as_bytes().to_vec(),
        };

        let inode_data = self.read_inode(inode).ok_or(super::Error::FileNotFound)?;
        let file = self.read_file(inode_data)?;

        let mut entries = Vec::new();
        let mut offset = 0;
        while offset < file.len() {
            let x = &mut file[offset..].iter();
            let mut entry = DirectoryEntry::from_buffer(x).unwrap();
            if entry.inode == 0 {
                new_entry.entry_size = (entries
                .last()
                .map(|x| x.entry_size as usize)
                .copied()
                .unwrap_or(0) + core::mem::size_of::<DirectoryEntry>()) as u32;
                entry = new_entry;
                entries.push(entry);

            }
            entries.push(entry.clone());
            offset += entry.entry_size as usize;
        }


        for entry in entries {

        }


    }
    

    pub fn write_file(
        &mut self,
        parent_inode_number: u32,
        name: &str,
        data: &[u8],
    ) -> Result<(), super::Error> {
        let mut inode = Inode::new();
        inode.type_and_perm = INODE_TYPE_REG | 0o644;
        inode.size_low = data.len() as u32;
        inode.last_access_time = 1744108895;
        inode.last_modif_time = 1744108895;
        inode.creation_time = 1744108895;
        let mut remaining_size = data.len() as u32;
        let mut current_dbp = 0;
        while remaining_size != 0 {
            let current_block = self.allocate_block().ok_or(super::Error::NotEnoughSpace)?;
            inode.dbp[current_dbp] = current_block;
            let block_size = self.super_block.block_size();
            if remaining_size < block_size as u32 {
                self.write_block(current_block, &data[..remaining_size as usize]);
                remaining_size = 0;
                break;
            } else {
                self.write_block(current_block, &data[..block_size as usize]);
                remaining_size -= block_size as u32;
            }
            current_dbp += 1;
        }

        inode.block_addr = current_dbp as u32;
        if remaining_size != 0 {
            let current_block = self.allocate_block().ok_or(super::Error::NotEnoughSpace)?;
            inode.sibp = current_block;
            let block_size = self.super_block.block_size();
            if remaining_size < block_size as u32 {
                self.write_block(current_block, &data[..remaining_size as usize]);
                remaining_size = 0;
            } else {
                self.write_block(current_block, &data[..block_size as usize]);
                remaining_size -= block_size as u32;
            }

            if remaining_size != 0 {
                let current_block = self.allocate_block().ok_or(super::Error::NotEnoughSpace)?;
                inode.dibp = current_block;
                let block_size = self.super_block.block_size();
                if remaining_size < block_size as u32 {
                    self.write_block(current_block, &data[..remaining_size as usize]);
                    remaining_size = 0;
                } else {
                    self.write_block(current_block, &data[..block_size as usize]);
                    remaining_size -= block_size as u32;
                }

                if remaining_size != 0 {
                    let current_block =
                        self.allocate_block().ok_or(super::Error::NotEnoughSpace)?;
                    inode.tibp = current_block;
                    let block_size = self.super_block.block_size();
                    if remaining_size < block_size as u32 {
                        self.write_block(current_block, &data[..remaining_size as usize]);
                        remaining_size = 0;
                    } else {
                        self.write_block(current_block, &data[..block_size as usize]);
                        remaining_size -= block_size as u32;
                    }

                    if remaining_size != 0 {
                        return Err(super::Error::NotEnoughSpace);
                    }
                }
            }
        }

        self.insert_inode(parent_inode_number, name, inode)?;

        Ok(())
    }

    pub fn write_directory(&mut self, parent_inode_number: u32, name: &str) -> Result<(), super::Error> {
        let mut inode = Inode::new();
        inode.type_and_perm = INODE_TYPE_DIR;
        inode.size_low = 0;
        inode.hard_link_count = 2; // . for "." and ".."

        self.insert_inode(parent_inode_number, name, inode)?;

        Ok(())
    }

    pub fn write_file_path(&mut self, path: super::Path, data: &[u8]) -> Result<(), super::Error> {
        if path.is_relative() {
            return Err(super::Error::CantUseRelPath);
        }


        let comps = path.components();

        let mut current_inode = 2;

        for (i, comp) in comps.iter().enumerate() {
            if i == comps.len() - 1 {
                break;
            }
            current_inode = self.read_inode_by_name(current_inode, comp)?;
            let inode = self.read_inode(current_inode).ok_or(super::Error::FileNotFound)?;
            if !inode.is_dir() && i != comps.len() - 1 {
                return Err(super::Error::NotADirectory);
            }
        }
        let inode = self.read_inode(current_inode).ok_or(super::Error::FileNotFound)?;

        if !inode.is_dir() {
            return Err(super::Error::NotADirectory);
        }

        self.write_file(current_inode, comps.last().unwrap(), data)?;

        self.flush();

        Ok(())
    }

    pub fn write_directory_path(&mut self, path: super::Path) -> Result<(), super::Error> {
        if path.is_relative() {
            return Err(super::Error::CantUseRelPath);
        }

        let comps = path.components();

        let mut current_inode = 2;

        for (i, comp) in comps.iter().enumerate() {
            current_inode = self.read_inode_by_name(current_inode, comp)?;
            let inode = self.read_inode(current_inode).ok_or(super::Error::FileNotFound)?;
            if !inode.is_dir() && i != comps.len() - 1 {
                return Err(super::Error::NotADirectory);
            }
        }

        let inode = self.read_inode(current_inode).ok_or(super::Error::FileNotFound)?;

        if !inode.is_dir() {
            return Err(super::Error::NotADirectory);
        }

        self.write_directory(current_inode, comps.last().unwrap())?;

        Ok(())
    }

    pub fn flush(&mut self) {
        for bgd in &self.bgd {
            bgd.flush(self.super_block, &mut self.disk);
            // flush inode table
        }
    }
}

impl<T: Disk + Clone> super::FileSystem for Ext2FS<T> {
    fn read(&mut self, path: super::Path) -> Result<Vec<u8>, super::Error> {
        let inode_id = self.read_path(path)?;
        let inode = self.read_inode(inode_id).ok_or(super::Error::FileNotFound)?;
        let file = self.read_file(inode)?;

        Ok(file)
    }

    fn write(&mut self, path: super::Path, data: &[u8]) -> Result<(), super::Error> {
        self.write_file_path(path, data)
    }

    fn delete(&mut self, path: super::Path) -> Result<(), super::Error> {
        todo!()
    }

    fn rename(&mut self, old_path: super::Path, new_path: super::Path) -> Result<(), super::Error> {
        todo!()
    }

    fn create_directory(&mut self, path: super::Path) -> Result<(), super::Error> {
        self.write_directory_path(path)
    }

    fn create_file(&mut self, path: super::Path) -> Result<(), super::Error> {
        todo!()
    }
}
