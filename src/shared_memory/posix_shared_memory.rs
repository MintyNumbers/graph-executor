use super::{rwlock, semaphore::Semaphore};
use anyhow::{anyhow, Result};
use iceoryx2_bb_container::semantic_string::SemanticString;
use iceoryx2_bb_system_types::file_name::FileName;
use iceoryx2_cal::{
    dynamic_storage::DynamicStorage, dynamic_storage::DynamicStorageBuilder,
    named_concept::NamedConceptBuilder,
};
use std::{fmt::Debug, sync::atomic::AtomicU8, sync::atomic::Ordering, usize};

pub struct PosixSharedMemory<S: DynamicStorage<AtomicU8>> {
    /// Prefix of all shared memory storages in `/dev/shm`
    filename_prefix: String,
    /// Write lock, 1: no current writer, 0: currently active writer
    write_lock: Semaphore,
    /// Number of current readers
    read_count: Semaphore,
    /// Keep alive so that the storage is not discarded
    data_storages: Vec<S>,
}

impl<S> std::fmt::Debug for PosixSharedMemory<S>
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

impl<S: DynamicStorage<AtomicU8>> PosixSharedMemory<S> {
    /// Create new Iox2ShmMapping with n storages with filename_prefix.
    pub fn new(filename_prefix: &str, data: impl serde::Serialize + Debug) -> Result<Self> {
        let filename_prefix = filename_prefix.replace("/", "_"); // Handle slash in filename

        // Create RwLock, construct shared memory mapping
        let write_lock = Semaphore::create(&format!("/{}_write_lock", filename_prefix), 1)
            .map_err(|e| anyhow!("Failed to create write_lock: {}", e))?;
        let read_count = Semaphore::create(&format!("/{}_read_count", filename_prefix), 0)
            .map_err(|e| anyhow!("Failed to create read_count: {}", e))?;

        let mut shm_mapping = PosixSharedMemory {
            filename_prefix,
            write_lock,
            read_count,
            data_storages: vec![],
        };

        // Initial write of data to shared memory
        shm_mapping.write_to_shm(&data)?;

        Ok(shm_mapping)
    }

    /// Create Iox2ShmMapping from storages with filename_prefix that already exist in shared memory.
    pub fn open<T: serde::de::DeserializeOwned>(filename_prefix: &str) -> Result<(Self, T)> {
        let filename_prefix = filename_prefix.replace("/", "_"); // Handle slash in filename

        // Read semaphores from shared memory, construct shared memory mapping
        let write_lock = Semaphore::open(&format!("/{}_write_lock", filename_prefix))
            .map_err(|e| anyhow!("Failed to open write_lock: {}", e))?;
        let read_count = Semaphore::open(&format!("/{}_read_count", filename_prefix))
            .map_err(|e| anyhow!("Failed to open read_count: {}", e))?;

        let mut shm_mapping = PosixSharedMemory {
            filename_prefix,
            write_lock,
            read_count,
            data_storages: vec![],
        };

        // Acquire read lock
        rwlock::read_lock(&shm_mapping.write_lock, &shm_mapping.read_count)?;

        // Read data bytes from shared memory
        let data_bytes = shm_mapping.read_from_shm()?;

        // Release read lock
        rwlock::read_unlock(&shm_mapping.read_count)?;

        // Deserialize and return data
        let data = rmp_serde::from_slice::<T>(&data_bytes)?;
        Ok((shm_mapping, data))
    }

    /// Acquire read lock, serialize read data from existing storages, deserialize it and write to `self.data`.
    pub fn read<T: serde::de::DeserializeOwned>(&mut self) -> Result<T> {
        // Acquire read lock
        self.read_lock()?;

        // Read data from shared memory
        let data_bytes = self.read_from_shm()?;

        // Release read lock
        self.read_unlock()?;

        // Return deserialized data
        let data = rmp_serde::from_slice::<T>(data_bytes.as_slice())?;
        Ok(data)
    }

    /// Acquire write lock and write `data` to shared memory.
    /// Storages are defined by `self.filename_prefix` and new storages are created if necessary / old storages are deleted if no longer necessary.
    pub fn write<T: serde::Serialize>(&mut self, data: &T) -> Result<()> {
        // Acquire write lock
        self.write_lock()?;

        // Initialize data for write
        self.write_to_shm(data)?;

        // Release write lock
        self.write_unlock()?;

        Ok(())
    }

    /// Acquire write lock, write `data_write` to shared memory if `data_condition` is equal to current data in shared memory.
    /// If `data_condition` is not equal to the data in shared memory, then return the data in shared memory.
    pub fn shm_compare_data_and_swap<
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    >(
        &mut self,
        data_equal_to_shm: &T,
        data_write: &T,
    ) -> Result<Option<T>> {
        // Acquire exclusive (write) lock
        self.write_lock()?;

        // Write data to shared memory if `data_condition` is equal to current state of data in shared memory
        let data_bytes = self.read_from_shm()?;
        let data_in_shm = rmp_serde::from_slice::<T>(data_bytes.as_slice())?;
        match data_in_shm == *data_equal_to_shm {
            true => {
                // Release write lock and return None on successful write
                self.write_to_shm(data_write)?;
                self.write_unlock()?;
                return Ok(None);
            }
            false => {
                // Release write lock and if `data_condition` no longer matches return `data_in_shm`
                self.write_unlock()?;
                return Ok(Some(data_in_shm));
            }
        }
    }

    /// Acquire read lock on shared memory storages.
    pub(crate) fn read_lock(&mut self) -> Result<()> {
        rwlock::read_lock(&self.write_lock, &self.read_count)
    }

    /// Release read lock on shared memory storages.
    pub(crate) fn read_unlock(&mut self) -> Result<()> {
        rwlock::read_unlock(&self.read_count)
    }

    /// Acquire write lock on shared memory storages.
    pub(crate) fn write_lock(&mut self) -> Result<()> {
        rwlock::write_lock(&self.write_lock, &self.read_count)
    }

    /// Release write lock on shared memory storages.
    pub(crate) fn write_unlock(&mut self) -> Result<()> {
        rwlock::write_unlock(&self.write_lock)
    }

    /// Returns `data_bytes` from storages defined by `filename_prefix` and writes `data_storages` to `self`.
    pub(crate) fn read_from_shm(&mut self) -> Result<Vec<u8>> {
        let mut bytes = vec![];

        // Read total buffer length from shared memory
        let usize_buf_len = usize::MAX.to_be_bytes().len();
        for offset in 0..usize_buf_len {
            match &self.data_storages.get(offset) {
                // Read storages from `self`
                Some(storage) => bytes.push(storage.get().load(Ordering::Relaxed)),
                None => {
                    let storage_name: FileName =
                        FileName::new(format!("{}_{}", &self.filename_prefix, offset).as_bytes())?;
                    match S::Builder::new(&storage_name).open() {
                        Err(e) => panic!("Failed to open existing DynamicStorage: {:?}", e),
                        Ok(s) => {
                            bytes.push(s.get().load(Ordering::Relaxed));
                            self.data_storages.push(s);
                        }
                    };
                }
            }
        }

        // Read all data from shared memory
        let total_buf_len = usize::from_be_bytes(bytes[0..usize_buf_len].try_into()?); // Number of storages containing relevant data
        for offset in usize_buf_len..total_buf_len {
            match &self.data_storages.get(offset) {
                // Read storages from `self`
                Some(storage) => bytes.push(storage.get().load(Ordering::Relaxed)),
                // Construct new storages if there are more allocated in shared memory/to match total_buf_len
                None => {
                    let storage_name: FileName =
                        FileName::new(format!("{}_{}", &self.filename_prefix, offset).as_bytes())?;
                    match S::Builder::new(&storage_name).open() {
                        Err(e) => panic!(
                            "Failed to open existing DynamicStorage {}: {:?}",
                            storage_name, e
                        ),
                        Ok(s) => {
                            bytes.push(s.get().load(Ordering::Relaxed));
                            self.data_storages.push(s);
                        }
                    };
                }
            }
        }

        // Remove storages if the data in the shared memory now requires fewer storages.
        while total_buf_len < self.data_storages.len() {
            self.data_storages
                .pop()
                .ok_or(anyhow!("No DynamicStorage despite successful check."))?
                .acquire_ownership(); // underlying storage resources are dropped on scope end
        }

        // Return data bytes
        Ok(bytes[usize_buf_len..total_buf_len].to_vec())
    }

    /// Writes supplied bytes to either the `data_storages` or `lock_storages` in `Self`.
    /// Argument `data` determines whether `self.data` or `self.lock` will be written to shared memory.
    pub(crate) fn write_to_shm<T: serde::Serialize>(&mut self, data: &T) -> Result<()> {
        let bytes = {
            let data_bytes = rmp_serde::to_vec(&data)?; // Serialized data bytes to be written in `data_storages`
            let usize_buf_len = usize::MAX.to_be_bytes().len(); // Number of storages (number of bytes) required for a single usize as bytes
            let total_buf_len = usize_buf_len + data_bytes.len(); // Total amount of data_storages (number of bytes)
            let mut total_buf_len_bytes = total_buf_len.to_be_bytes().to_vec(); // Total number of storages (stays constant despite value change)

            // Bytes that will be written (total_buf_len and data) are simply concatenated
            total_buf_len_bytes.extend(data_bytes);
            total_buf_len_bytes
        };

        // Write to shared memory
        let mut offset = 0;
        for byte in bytes {
            match &self.data_storages.get(offset) {
                // Write to existing storages
                Some(storage) => storage.get().store(byte, Ordering::Relaxed),
                // Create new storages if data to be written requires more space than currently allocated
                None => {
                    let storage_name: FileName =
                        FileName::new(format!("{}_{}", &self.filename_prefix, offset).as_bytes())?;
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
            self.data_storages
                .pop()
                .ok_or(anyhow!("No DynamicStorage despite successful check."))?
                .acquire_ownership(); // underlying storage resources are dropped on scope end
        }

        assert_eq!(self.data_storages.len(), offset);

        Ok(())
    }
}
