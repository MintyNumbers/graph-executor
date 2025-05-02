use anyhow::{anyhow, Result};
use iceoryx2_bb_container::semantic_string::SemanticString;
use iceoryx2_bb_system_types::file_name::FileName;
use iceoryx2_cal::dynamic_storage::{DynamicStorage, DynamicStorageBuilder, DynamicStorageOpenError};
use iceoryx2_cal::named_concept::NamedConceptBuilder;
use std::sync::atomic::AtomicU32;
use std::{sync::atomic::AtomicU8, sync::atomic::Ordering};

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

// TODO: create lockfile to handle process death

#[derive(Debug)]
pub struct Iox2ShmMapping<S, T>
where
    for<'a> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'a>,
    S: DynamicStorage<AtomicU8>,
{
    // buf_len: usize,
    filename_prefix: String, // Prefix of all storages in shared memory
    lock: AtomicU32,         // C-style RwLock, u32::MAX indicates write lock
    lock_storages: Vec<S>,   // Keep alive so that the storage is not discarded
    pub data: T,             // Data stored in shared memory
    data_storages: Vec<S>,   // Keep alive so that the storage is not discarded
}

impl<S, T> Iox2ShmMapping<S, T>
where
    for<'a> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'a>,
    S: DynamicStorage<AtomicU8>,
{
    /// Create new Iox2ShmMapping with n storages with filename_prefix.
    pub fn new<'a>(filename_prefix: String, data: T) -> Result<Self> {
        // Initial write of lock to shared memory
        let lock = AtomicU32::new(0);
        let mut offset = 0;
        let mut lock_storages: Vec<S> = vec![];
        let lock_bytes = rmp_serde::to_vec(&lock)?;

        for byte in lock_bytes.as_slice() {
            let storage_name: FileName = FileName::new(format!("{}_lock_{}", filename_prefix, offset).as_bytes())?;
            let storage = S::Builder::new(&storage_name)
                .create(AtomicU8::new(0))
                .map_err(|e| anyhow!("Failed to create new shared memory Storage: {:?}", e))?;
            storage.get().store(*byte, Ordering::Relaxed);

            lock_storages.push(storage);
            offset += 1;
        }

        // Initial write of data to shared memory
        let mut offset = 0;
        let mut data_storages: Vec<S> = vec![];
        let data_bytes = rmp_serde::to_vec(&data)?;

        for byte in data_bytes.as_slice() {
            let storage_name: FileName = FileName::new(format!("{}_data_{}", filename_prefix, offset).as_bytes())?;
            let storage = S::Builder::new(&storage_name)
                .create(AtomicU8::new(0))
                .map_err(|e| anyhow!("Failed to create new shared memory Storage: {:?}", e))?;
            storage.get().store(*byte, Ordering::Relaxed);

            data_storages.push(storage);
            offset += 1;
        }

        println!("lock: {:?}\nlock_bytes: {:?}", lock, lock_bytes.as_slice());
        println!("data: {:?}\ndata_bytes: {:?}", data, data_bytes.as_slice());

        Ok(Iox2ShmMapping {
            filename_prefix,
            lock,
            lock_storages,
            data,
            data_storages,
        })
    }

    /// Create Iox2ShmMapping from storages with filename_prefix that already exist in shared memory.
    pub fn open_existing(filename_prefix: String) -> Result<Self> {
        // Read and deserialize lock bytes from shared memory
        let (lock_bytes, lock_storages) = Iox2ShmMapping::<S, T>::read_from_shm_by_filename(&filename_prefix, false)?;
        let lock: AtomicU32 = rmp_serde::from_slice(&lock_bytes)?;

        // Acquire read lock for data bytes in shared memory
        // TODO
        // self.read_lock; lock reading and deserializing will be moved to read_lock()

        // Read and deserialize data bytes from shared memory
        let (data_bytes, data_storages) = Iox2ShmMapping::<S, T>::read_from_shm_by_filename(&filename_prefix, true)?;
        let data: T = rmp_serde::from_slice(&data_bytes)?;

        println!("lock: {:?}\nlock_bytes: {:?}", lock, lock_bytes);
        println!("data: {:?}\ndata_bytes: {:?}", data, data_bytes);

        Ok(Iox2ShmMapping {
            filename_prefix,
            lock,
            lock_storages,
            data_storages,
            data,
        })
    }

    /// Acquire write lock, serialize `self.data` and write it to existing storages.
    /// Storages are defined by `self.filename_prefix` and new storages are created if necessary / old storages are deleted if no longer necessary.
    pub fn write_self_to_shm(&mut self) -> Result<()> {
        // Acquire write lock
        // TODO
        // self.write_lock; lock reading and deserializing will be in write_lock()

        // Initialize data for write
        self.write_to_shm_by_filename(true)?;

        println!("self.data: {:?}", self.data);

        Ok(())
    }

    // fn read_lock(&self) {}

    // fn write_lock(&self) {}

    /// Returns `data` or `lock` bytes from storages defined by `filename_prefix`.
    fn read_from_shm_by_filename(filename_prefix: &str, data: bool) -> Result<(Vec<u8>, Vec<S>)> {
        let mut offset = 0;
        let mut bytes = vec![];
        let mut storages = vec![];
        loop {
            let storage_name: FileName = FileName::new(format!("{}_{}_{}", filename_prefix, if data { "data" } else { "lock" }, offset).as_bytes())?;
            let storage = match S::Builder::new(&storage_name).open() {
                Err(DynamicStorageOpenError::DoesNotExist) => break, // Break once all existing storages have been read
                Err(e) => panic!("Failed to open existing DynamicStorage: {:?}", e),
                Ok(s) => s,
            };

            bytes.push(storage.get().load(Ordering::Relaxed));
            storages.push(storage);
            offset += 1;
        }

        Ok((bytes, storages))
    }

    /// Writes supplied bytes to either the `data_storages` or `lock_storages` in `Self`.
    /// Argument `data` determines whether `self.data` or `self.lock` will be written to shared memory.
    fn write_to_shm_by_filename(&mut self, data: bool) -> Result<()> {
        let mut offset = 0;
        let (storages, bytes) = if data {
            (&mut self.data_storages, rmp_serde::to_vec(&self.data)?) // Data storages and bytes to be written in these storages
        } else {
            (&mut self.lock_storages, rmp_serde::to_vec(&self.lock)?) // Lock storages and bytes to be written in these storages
        };

        // Write to existing shared memory
        for byte in bytes {
            match storages.get(offset) {
                // Write to existing storages
                Some(storage) => storage.get().store(byte, Ordering::Relaxed),
                // Create new storages if data to be written requires more space than the previously stored data
                None => {
                    let storage_name: FileName =
                        FileName::new(format!("{}_{}_{}", &self.filename_prefix, if data { "data" } else { "lock" }, offset).as_bytes())?;
                    let storage = S::Builder::new(&storage_name)
                        .create(AtomicU8::new(0))
                        .map_err(|e| anyhow!("Failed to create new DynamicStorage: {:?}", e))?;
                    storage.get().store(byte, Ordering::Relaxed);

                    (*storages).push(storage);
                }
            }
            offset += 1;
        }
        // Remove storages if data to be written requires less space than the previously stored data
        while storages.len() - offset > 0 {
            let storage = storages.pop().ok_or(anyhow!("No DynamicStorage despite successful check."))?;
            storage.acquire_ownership(); // is dropped on scope end
        }

        assert_eq!(storages.len(), offset);

        Ok(())
    }
}

/*
// Read Guard for Iox2ShmMapping<S, T>
pub struct ReadGuard<'a, S, T>
where
    for<'b> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'b>,
    S: DynamicStorage<AtomicU8>,
{
    rwlock: &'a Iox2ShmMapping<S, T>,
}

impl<S, T> std::ops::Deref for ReadGuard<'_, S, T>
where
    for<'a> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'a>,
    S: DynamicStorage<AtomicU8>,
{
    type Target = Iox2ShmMapping<S, T>;
    fn deref(&self) -> &Iox2ShmMapping<S, T> {
        self.rwlock
    }
}

impl<S, T> std::ops::Drop for ReadGuard<'_, S, T>
where
    for<'a> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'a>,
    S: DynamicStorage<AtomicU8>,
{
    fn drop(&mut self) {
        let (lock_bytes, lock_storages) = Iox2ShmMapping::<S, T>::read_from_shm_by_filename(&self.rwlock.filename_prefix, false).unwrap();
        let lock: AtomicU32 = rmp_serde::from_slice(&lock_bytes).unwrap();

        if lock.fetch_sub(1, Ordering::Release) == 1 {
            // Wake up a waiting writer, if any.
            // TODO
        }
    }
}

// Write Guard for Iox2ShmMapping<S, T>
pub struct WriteGuard<'a, S, T>
where
    for<'b> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'b>,
    S: DynamicStorage<AtomicU8>,
{
    rwlock: &'a Iox2ShmMapping<S, T>,
}

impl<S, T> std::ops::Deref for WriteGuard<'_, S, T>
where
    for<'a> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'a>,
    S: DynamicStorage<AtomicU8>,
{
    type Target = Iox2ShmMapping<S, T>;
    fn deref(&self) -> &Iox2ShmMapping<S, T> {
        self.rwlock
    }
}

impl<S, T> Drop for WriteGuard<'_, S, T>
where
    for<'b> T: std::fmt::Debug + serde::Serialize + serde::Deserialize<'b>,
    S: DynamicStorage<AtomicU8>,
{
    fn drop(&mut self) {
        let (lock_bytes, _lock_storages) = Iox2ShmMapping::<S, T>::read_from_shm_by_filename(&self.rwlock.filename_prefix, false).unwrap();
        let lock: AtomicU32 = rmp_serde::from_slice(&lock_bytes).unwrap();

        lock.store(0, Ordering::Release);
        // Wake up all waiting readers and writers.
        // TODO

        // Atomically write to lock in shared memory... => lock is already there to allow for atomic operations on the data - this sounds recursive
        // TODO
    }
}
*/
