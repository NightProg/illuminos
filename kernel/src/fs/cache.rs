use alloc::collections::btree_map::BTreeMap;

pub struct PageCache {
    pages: BTreeMap<u64, CachedPage>,
}

pub struct CachedPage {
    data: [u8; 4096],
    dirty: bool,
}
