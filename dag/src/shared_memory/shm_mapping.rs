use super::{rwlock, semaphore::Semaphore};
use anyhow::{anyhow, Result};
use iceoryx2_bb_container::semantic_string::SemanticString;
use iceoryx2_bb_system_types::file_name::FileName;
use iceoryx2_cal::{
    dynamic_storage::{DynamicStorage, DynamicStorageBuilder, DynamicStorageOpenError},
    named_concept::NamedConceptBuilder,
};
use std::{fmt::Debug, sync::atomic::AtomicU8, sync::atomic::Ordering};

// Findings:
// - shared memory closes on scope end; it does not close on Ctrl + C
// - to keep the mapping alive the associated `Shm` can't be deconstructed
// - each time i create a new `Shm` it gets a new payload_start_address
// - creating `Shm` in one process and opening it in another results in an "off" start address
//   - after each read the offset becomes bigger
//   - solution: imma just do n `DynamicStorage`s for now
// - Segmentation fault (core dumped) when trying to cast byte array as `DirectedAcyclicGraph`
//   - no segfault inside the process which created the graph
//   - suggests that the graph structure depends on something more (which is not translated into the byte array representation)
//   - solution: serialization...
// - `DynamicStorage` uses `Atomic`s due to no method giving an exclusive reference => `Atomic`s' interior mutability is necessary
// - infinite loop when trying to serialize the RwLock/Mutex after acquiring lock or when trying to acquire non-released lock

pub struct Iox2ShmMapping<S>
where
    S: DynamicStorage<AtomicU8>,
{
    // buf_len: usize,       // Length of serialized data in bytes
    filename_prefix: String, // Prefix of all storages in shared memory
    write_lock: Semaphore,   // Write lock, 1: no current writer, 0: currently active writer
    read_count: Semaphore,   // Number of current readers
    data_storages: Vec<S>,   // Keep alive so that the storage is not discarded
}

impl<S> std::fmt::Debug for Iox2ShmMapping<S>
where
    S: DynamicStorage<AtomicU8>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Iox2ShmMapping: {{filename_prefix: {:?}, write_lock: {:?}, read_count: {:?}, data_storages: {:?}}}",
            self.filename_prefix, self.write_lock, self.read_count, self.data_storages
        )
    }
}

impl<S> Iox2ShmMapping<S>
where
    S: DynamicStorage<AtomicU8>,
{
    /// Create new Iox2ShmMapping with n storages with filename_prefix.
    pub fn new(filename_prefix: String, data: impl serde::Serialize + Debug) -> Result<Self> {
        let filename_prefix = filename_prefix.replace("/", "_"); // Handle slash in filename

        // Initial write of data to shared memory
        let mut offset = 0;
        let mut data_storages: Vec<S> = vec![];
        let data_bytes = rmp_serde::to_vec(&data)?;
        for byte in data_bytes.as_slice() {
            let storage_name: FileName = FileName::new(format!("{}_{}", filename_prefix, offset).as_bytes())?;
            let storage = S::Builder::new(&storage_name)
                .create(AtomicU8::new(0))
                .map_err(|e| anyhow!("Failed to create new shared memory Storage: {:?}", e))?;
            storage.get().store(*byte, Ordering::Relaxed);

            data_storages.push(storage);
            offset += 1;
        }

        // Create RwLock
        let write_lock = Semaphore::create(&format!("/{}_write_lock_write", filename_prefix), 1).map_err(|e| anyhow!("Failed to create write_lock: {}", e))?;
        let read_count = Semaphore::create(&format!("/{}_read_count_write", filename_prefix), 0).map_err(|e| anyhow!("Failed to create read_count: {}", e))?;

        println!("data: {:?}\ndata_bytes: {:?}", data, data_bytes.as_slice());

        Ok(Iox2ShmMapping {
            filename_prefix,
            write_lock,
            read_count,
            data_storages,
        })
    }

    /// Create Iox2ShmMapping from storages with filename_prefix that already exist in shared memory.
    pub fn open<T: Debug + serde::de::DeserializeOwned>(filename_prefix: String) -> Result<(Self, T)> {
        // Read semaphore from shared memory and acquire read lock
        let write_lock = Semaphore::open(&format!("/{}_write_lock_write", filename_prefix)).map_err(|e| anyhow!("Failed to open write_lock: {}", e))?;
        let read_count = Semaphore::open(&format!("/{}_read_count_write", filename_prefix)).map_err(|e| anyhow!("Failed to open read_count: {}", e))?;
        rwlock::read_lock(&write_lock, &read_count)?;

        // Read data bytes from shared memory
        let (data_bytes, data_storages) = Iox2ShmMapping::<S>::read_from_shm_by_filename(&filename_prefix)?;

        // Release read lock
        rwlock::read_unlock(&read_count)?;

        // Deserialize data
        let data = rmp_serde::from_slice::<T>(&data_bytes)?;

        println!("write_lock: {:?}\tread_count: {:?}", write_lock.get_value(), read_count.get_value());
        println!("data: {:?}\ndata_bytes: {:?}", data, data_bytes);

        Ok((
            Iox2ShmMapping {
                filename_prefix,
                write_lock,
                read_count,
                data_storages,
            },
            data,
        ))
    }

    /// Acquire read lock, serialize read data from existing storages, deserialize it and write to `self.data`.
    pub fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        // Acquire read lock
        rwlock::read_lock(&self.write_lock, &self.read_count)?;

        // Read, deserialize and write data to self
        let (data_bytes, _) = Iox2ShmMapping::<S>::read_from_shm_by_filename(&self.filename_prefix)?;
        let data: T = rmp_serde::from_slice::<T>(data_bytes.as_slice())?;

        println!("Read Locked...");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Release read lock
        rwlock::read_unlock(&self.read_count)?;

        Ok(data)
    }

    /// Acquire write lock, serialize `self.data` and write it to existing storages.
    /// Storages are defined by `self.filename_prefix` and new storages are created if necessary / old storages are deleted if no longer necessary.
    pub fn write<T: serde::Serialize>(&mut self, data: &T) -> Result<()> {
        // Acquire write lock
        rwlock::write_lock(&self.write_lock, &self.read_count)?;

        // Initialize data for write
        self.write_to_shm_by_filename(data)?;

        println!("Write Locked...");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Release write lock
        rwlock::write_unlock(&self.write_lock)?;

        Ok(())
    }

    /// Returns `data` or `lock` bytes from storages defined by `filename_prefix`.
    fn read_from_shm_by_filename(filename_prefix: &str) -> Result<(Vec<u8>, Vec<S>)> {
        let mut offset = 0;
        let mut data_bytes = vec![];
        let mut data_storages = vec![];
        'x: loop {
            let storage_name: FileName = FileName::new(format!("{}_{}", filename_prefix, offset).as_bytes())?;
            let storage = match S::Builder::new(&storage_name).open() {
                Err(DynamicStorageOpenError::DoesNotExist) => break 'x, // Break once all existing storages have been read
                Err(e) => panic!("Failed to open existing DynamicStorage: {:?}", e),
                Ok(s) => s,
            };

            data_bytes.push(storage.get().load(Ordering::Relaxed));
            data_storages.push(storage);
            offset += 1;
        }

        Ok((data_bytes, data_storages))
    }

    /// Writes supplied bytes to either the `data_storages` or `lock_storages` in `Self`.
    /// Argument `data` determines whether `self.data` or `self.lock` will be written to shared memory.
    fn write_to_shm_by_filename<T: serde::Serialize>(&mut self, data: &T) -> Result<()> {
        let mut offset = 0;
        let data_bytes = rmp_serde::to_vec(&data)?; // Serialized data bytes to be written in `data_storages`

        // Write to existing shared memory
        for byte in data_bytes {
            match &self.data_storages.get(offset) {
                // Write to existing storages
                Some(storage) => storage.get().store(byte, Ordering::Relaxed),
                // Create new storages if data to be written requires more space than the previously stored data
                None => {
                    let storage_name: FileName = FileName::new(format!("{}_{}", &self.filename_prefix, offset).as_bytes())?;
                    let storage = S::Builder::new(&storage_name)
                        .create(AtomicU8::new(0))
                        .map_err(|e| anyhow!("Failed to create new DynamicStorage: {:?}", e))?;
                    storage.get().store(byte, Ordering::Relaxed);

                    self.data_storages.push(storage);
                }
            }
            offset += 1;
        }
        // Remove storages if data to be written requires less space than the previously stored data
        while &self.data_storages.len() - offset > 0 {
            let storage = &self.data_storages.pop().ok_or(anyhow!("No DynamicStorage despite successful check."))?;
            storage.acquire_ownership(); // is dropped on scope end
        }

        assert_eq!(self.data_storages.len(), offset);

        Ok(())
    }
}
