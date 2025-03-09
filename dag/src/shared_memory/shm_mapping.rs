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
    pub buf_len: usize,
    pub wrapped: T,
    pub serialize: bool,
}

impl<T> ShmMapping<T>
where
    T: Display + AsFromBytes + serde::Serialize + serde::de::DeserializeOwned,
{
    /// Creates a new shared memory mapping, writes `wrapped` to it, initializes `Mutex` and returns `ShmMapping`.
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

        // Initialize `raw_ptr`.
        let mut raw_ptr: *mut u8 = shmem.as_ptr();
        let is_init: &mut AtomicU8;
        unsafe {
            is_init = &mut *(raw_ptr as *mut u8 as *mut AtomicU8);
            raw_ptr = raw_ptr.add(8);
        };

        // Initialize `raw_sync::locks::Mutex` (cross-process synchronisation).
        is_init.store(0, Ordering::Relaxed);
        unsafe {
            raw_sync::locks::Mutex::new(
                raw_ptr,                                                     // Base address of Mutex.
                raw_ptr.add(raw_sync::locks::Mutex::size_of(Some(raw_ptr))), // Address of data protected by mutex.
            )
            .map_err(|e| anyhow!("Failed to initialize Mutex: {}", e))?
        };
        is_init.store(1, Ordering::Relaxed);

        // Write `wrapped` to shared memory mapping.
        shm_mapping.write_to_shared_memory(is_init, raw_ptr)?;

        // Return `ShmMapping`.
        Ok(shm_mapping)
    }

    /// Update `self.wrapped` field and the wrapped struct stored in the shared memory mapping with
    /// supplied by the `wrapped` argument.
    pub fn overwrite_shared_memory_data(&mut self, wrapped: T) -> Result<()> {
        // Update `wrapped` field in struct.
        self.wrapped = wrapped;

        // Update `wrapped` in shared memory mapping.
        let (is_init, raw_ptr) = self.open_shared_memory()?;
        self.write_to_shared_memory(is_init, raw_ptr)?;

        Ok(())
    }

    /// Open shared memory mapping and return `is_init` and `raw_ptr`.
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

    pub fn get_mutex(is_init: &mut AtomicU8, raw_ptr: *mut u8) -> Result<Box<dyn raw_sync::locks::LockImpl>> {
        // Wait for initialized mutex.
        while is_init.load(Ordering::Relaxed) != 1 {
            println!("This shouldn't happen?");
        }

        // Load existing mutex.
        let (mutex, _bytes_used) = unsafe {
            raw_sync::locks::Mutex::from_existing(
                raw_ptr,                                                     // Base address of Mutex
                raw_ptr.add(raw_sync::locks::Mutex::size_of(Some(raw_ptr))), // Address of data  protected by mutex
            )
            .map_err(|e| anyhow!("Error loading Mutex: {}", e))?
        };

        Ok(mutex)
    }

    /// Write `wrapped` struct bytes to shared memory.
    fn write_to_shared_memory(&self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let mutex = ShmMapping::<T>::get_mutex(is_init, raw_ptr)?;
        let guard = mutex.lock().map_err(|e| anyhow!("Error acquiring Mutex lock: {}", e))?;
        // let val: &mut u8 = unsafe { &mut **guard };

        // Write to shared memory mapping.
        unsafe {
            raw_ptr = raw_ptr.add(raw_sync::locks::Mutex::size_of(Some(raw_ptr))); // Move pointer to data address.

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
