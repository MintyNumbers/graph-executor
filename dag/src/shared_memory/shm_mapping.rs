use super::as_from_bytes::AsFromBytes;
use anyhow::{anyhow, Result};
use raw_sync::locks::LockInit;
use shared_memory::ShmemConf;
use std::{fmt::Display, sync::atomic::AtomicU8, sync::atomic::Ordering};

#[derive(Debug)]
pub struct ShmMapping<T>
where
    T: Display + AsFromBytes + serde::Serialize + serde::de::DeserializeOwned,
{
    shmem_flink: String,
    pub(crate) buf_len: usize,
    pub(crate) wrapped: T,
    pub(crate) serialize: bool,
}

impl<T> ShmMapping<T>
where
    T: Display + AsFromBytes + serde::Serialize + serde::de::DeserializeOwned,
{
    /// Creates a new shared memory mapping, writes `wrapped` to it, initializes `RwLock` over the shared memory
    /// mapping and returns `ShmMapping`.
    pub fn new(shmem_flink: String, wrapped: T, serialize: bool) -> Result<Self> {
        // Construct `ShmMapping`.
        let shm_mapping = ShmMapping {
            shmem_flink: shmem_flink,
            buf_len: if serialize {
                rmp_serde::to_vec(&wrapped)?.len()
            } else {
                wrapped.as_bytes().len()
            },
            wrapped: wrapped,
            serialize: serialize,
        };

        // Create or open the shared memory mapping
        // let _ = std::fs::remove_file(&shm_mapping.shmem_flink);
        let shmem = match ShmemConf::new().size(shm_mapping.buf_len).flink(&shm_mapping.shmem_flink).create() {
            Ok(m) => m,
            Err(shared_memory::ShmemError::LinkExists) => ShmemConf::new().flink(&shm_mapping.shmem_flink).open()?,
            Err(e) => {
                return Err(anyhow!("Unable to create new shared memory section {}: {}", &shm_mapping.shmem_flink, e));
            }
        };

        // Initialize `raw_ptr` to the base address of the shared memory mapping.
        let mut raw_ptr: *mut u8 = shmem.as_ptr();
        // Initialize `is_init` indicating the initialization status of the RwLock over the shared memory mapping.
        let is_init: &mut AtomicU8;
        unsafe {
            is_init = &mut *(raw_ptr as *mut u8 as *mut AtomicU8);
            raw_ptr = raw_ptr.add(8); // Move pointer to data address.
        };

        // Initialize `raw_sync::locks::RwLock` (cross-process synchronisation).
        is_init.store(0, Ordering::Relaxed);
        let (_rwlock, _bytes_used) = unsafe {
            raw_sync::locks::RwLock::new(
                raw_ptr,                                                      // Base address of RwLock.
                raw_ptr.add(raw_sync::locks::RwLock::size_of(Some(raw_ptr))), // Address of data protected by RwLock.
            )
            .map_err(|e| anyhow!("Failed to initialize RwLock: {}", e))?
        };
        is_init.store(1, Ordering::Relaxed);

        // Write `wrapped` to shared memory mapping.
        shm_mapping.write_to_shared_memory(is_init, raw_ptr)?;

        // Return `ShmMapping`.
        Ok(shm_mapping)
    }

    /// Update `self.wrapped` field and the wrapped struct stored in the shared memory mapping with
    /// the supplied `wrapped` argument.
    pub(crate) fn update_wrapped(&mut self, wrapped: T) -> Result<()> {
        // Update `wrapped` field in struct.
        self.wrapped = wrapped;

        // Update `wrapped` in shared memory mapping.
        let (is_init, raw_ptr) = self.open_shared_memory()?;
        self.write_to_shared_memory(is_init, raw_ptr)?;

        Ok(())
    }

    /// Open shared memory mapping and return `is_init` (RwLock initialization status) and `raw_ptr`
    /// (raw pointer to the shared memory mapping).
    fn open_shared_memory(&self) -> Result<(&mut AtomicU8, *mut u8)> {
        let shmem = ShmemConf::new()
            .flink(&self.shmem_flink)
            .open()
            .map_err(|e| anyhow!("Unable to open existing shared memory section {}: {}", &self.shmem_flink, e))?;

        let mut raw_ptr: *mut u8 = shmem.as_ptr();
        let is_init: &mut AtomicU8;
        unsafe {
            is_init = &mut *(raw_ptr as *mut u8 as *mut AtomicU8);
            raw_ptr = raw_ptr.add(8);
        };

        Ok((is_init, raw_ptr))
    }

    /// Takes a raw pointer to the shared memory mapping and an atomic indicating the initialization status
    /// of the RwLock over the shared memory mapping and returns a `RwLock` over the shared memory mapping.
    /// The `RwLock` returned by this function is not locked.
    pub(crate) fn get_rwlock(is_init: &mut AtomicU8, raw_ptr: *mut u8) -> Result<Box<dyn raw_sync::locks::LockImpl>> {
        // Wait for initialized RwLock.
        while is_init.load(Ordering::Relaxed) != 1 {
            println!("This shouldn't happen?");
        }

        // Load existing RwLock.
        let (rwlock, _bytes_used) = unsafe {
            raw_sync::locks::RwLock::from_existing(
                raw_ptr,                                                      // Base address of RwLock.
                raw_ptr.add(raw_sync::locks::RwLock::size_of(Some(raw_ptr))), // Address of data protected by RwLock.
            )
            .map_err(|e| anyhow!("Error loading RwLock: {}", e))?
        };

        Ok(rwlock)
    }

    /// Locks the RwLock over the shared memory mapping, writes `self.wrapped` bytes
    /// to the shared memory mapping and unlocks the RwLock.
    fn write_to_shared_memory(&self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let write_lock = ShmMapping::<T>::get_rwlock(is_init, raw_ptr)?;
        let guard = write_lock.lock().map_err(|e| anyhow!("Error acquiring RwLock: {}", e))?;
        // let val: &mut u8 = unsafe { &mut **guard };

        // Write to shared memory mapping.
        unsafe {
            raw_ptr = raw_ptr.add(raw_sync::locks::RwLock::size_of(Some(raw_ptr))); // Move pointer to data address.

            // Serialize wrapped struct or cast wrapped struct as bytes.
            let wrapped_bytes: Vec<u8> = if self.serialize {
                // rmp_serde::to_vec(&self.wrapped)?
                self.wrapped.as_bytes().to_vec()
            } else {
                self.wrapped.as_bytes().to_vec()
            };

            for byte in wrapped_bytes {
                raw_ptr.write(byte); // Write wrapped struct byte to memory.
                raw_ptr = raw_ptr.add(1); // Move pointer forward by the size of a byte.
            }
        }

        // Release lock over guarded data.
        drop(guard);

        Ok(())
    }
}
