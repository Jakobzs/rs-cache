use std::{ 
    collections::HashMap, 
    path::Path,
    fs::File,
    io::Read,
};
use rscache::{ 
    CacheCore, CacheRead, 
    Store,
    arc::Archive,
    idx::Index,
    error::ReadError,
    util,
};

fn main() -> rscache::Result<()> {
    let cache = CustomCache::new("./data/cache")?;
    let buffer = cache.read(255, 2)?;

    println!("buffer: {:?}", buffer);
    
    Ok(())
}

// CustomCache is essentially a Cache but its just to give you an
// idea of how to implement your own cache.
struct CustomCache {
    store: CustomStore,
    indices: HashMap<u8, Index>
}

// Identical to MemoryStore.
struct CustomStore {
    data: Vec<u8>
}

// If the Cache struct initializes correctly (for your use-case) but the supplied 
// stores (Memory- and FileStore) are not suitable you can simply only create
// a custom store. See the custom_store.rs example for more info.
impl CacheCore for CustomCache {
    fn new<P: AsRef<Path>>(path: P) -> rscache::Result<Self> {
        let path = path.as_ref();

        let store = util::load_store(path)?;
        let indices = util::load_indices(path)?;

        Ok(Self { store, indices })
    }
}

impl CacheRead for CustomCache {
    fn read(&self, index_id: u8, archive_id: u32) -> rscache::Result<Vec<u8>> {
        let index = match self.indices.get(&index_id) {
            Some(index) => index,
            None => return Err(ReadError::IndexNotFound(index_id).into())
        };

        let archive = match index.archives.get(&archive_id) {
            Some(archive) => archive,
            None => return Err(ReadError::ArchiveNotFound(index_id, archive_id).into())
        };

        Ok(self.store.read(&archive)?)
    }
}

impl Store for CustomStore {
    fn new(mut main_file: File) -> rscache::Result<Self> {
        let mut buffer = Vec::new();
        main_file.read_to_end(&mut buffer)?;
        
        Ok(Self { data: buffer })
    }

    fn read(&self, archive: &Archive) -> rscache::Result<Vec<u8>> {
        // Implement your own read here.

        println!("{:?}", archive);

        // For the sake of this example: only return the first
        // 25 bytes of the internal buffer.
        Ok(self.data[..25].to_vec())
    }
}