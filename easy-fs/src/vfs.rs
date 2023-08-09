use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    inode_id: u32,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

/// stat of Inode
pub struct Stat {
    /// is dir?
    pub is_dir: bool,
    /// is file?
    pub is_file: bool,
    /// inode id
    pub inode_id: u32,
    /// hard link num
    pub hard_link: u32,
}

// since easy-fs does not support shrink file size, in unlink, we set inode id of dirent to this inode_id
// this is the inode id of root inode
const DIRENT_INVALID_INODE_ID: u32 = 0;

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        inode_id: u32,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            inode_id,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name && dirent.inode_id() != DIRENT_INVALID_INODE_ID {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    inode_id,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            new_inode_id,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if dirent.inode_id() != DIRENT_INVALID_INODE_ID {
                    v.push(String::from(dirent.name()));
                }
            }
            v
        })
    }
    /// link old_name to new_name
    pub fn linkat(&self, old_name: &str, new_name: &str) -> isize {
        if old_name == new_name {
            return -1;
        }
        let mut fs = self.fs.lock();
        let old_inode_id = match self
            .read_disk_inode(|root_inode: &DiskInode| self.find_inode_id(old_name, root_inode))
        {
            Some(x) => x,
            None => return -1,
        };
        let (old_inode_block_id, old_inode_block_offset) = fs.get_disk_inode_pos(old_inode_id);
        get_block_cache(old_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(old_inode_block_offset, |old_inode: &mut DiskInode| {
                old_inode.hard_link += 1;
            });
        self.modify_disk_inode(|root_inode: &mut DiskInode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(new_name, old_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        block_cache_sync_all();
        0
    }
    /// unlink old_name
    pub fn unlinkat(&self, old_name: &str) -> isize {
        let fs = self.fs.lock();
        let old_inode_id = match self
            .read_disk_inode(|root_inode: &DiskInode| self.find_inode_id(old_name, root_inode))
        {
            Some(x) => x,
            None => return -1,
        };
        let (old_inode_block_id, old_inode_block_offset) = fs.get_disk_inode_pos(old_inode_id);
        let old_inode_hard_link =
            get_block_cache(old_inode_block_id as usize, Arc::clone(&self.block_device))
                .lock()
                .modify(old_inode_block_offset, |old_inode: &mut DiskInode| {
                    old_inode.hard_link -= 1;
                    old_inode.hard_link
                });
        self.modify_disk_inode(|root_inode: &mut DiskInode| {
            // remove file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    root_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                if dirent.name() == old_name && dirent.inode_id() != DIRENT_INVALID_INODE_ID {
                    let dirent = DirEntry::new(old_name, DIRENT_INVALID_INODE_ID);
                    root_inode.write_at(i * DIRENT_SZ, dirent.as_bytes(), &self.block_device);
                }
            }
        });
        if old_inode_hard_link == 0 {
            // remove file inode & data block
            drop(fs); // manually release lock
            let file_inode = Self::new(
                old_inode_block_id,
                old_inode_block_offset,
                old_inode_id,
                self.fs.clone(),
                self.block_device.clone(),
            );
            file_inode.clear();
        }
        block_cache_sync_all();
        0
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
    /// stat of inode
    pub fn stat(&self) -> Stat {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| Stat {
            is_dir: disk_inode.is_dir(),
            is_file: disk_inode.is_file(),
            inode_id: self.inode_id,
            hard_link: disk_inode.hard_link,
        })
    }
}
