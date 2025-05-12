use libc::{
    c_int, c_uint, mmap, munmap, sem_close, sem_open, sem_post, sem_trywait, sem_unlink, sem_wait, strerror, MAP_FAILED, MAP_SHARED, O_CREAT, PROT_READ,
    PROT_WRITE, SEM_FAILED, S_IRUSR, S_IWUSR,
};
use serde::{Deserialize, Serialize};
use std::{
    ffi::CStr,
    ffi::CString,
    fs::{remove_file, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    ptr, thread,
    time::Duration,
};

#[cfg(target_os = "macos")]
unsafe fn get_errno() -> i32 {
    *libc::__error()
}

#[cfg(target_os = "linux")]
unsafe fn get_errno() -> i32 {
    *libc::__errno_location()
}

/// Retrieves and formats an error message from `errno`.
fn get_last_error(context: &str) -> String {
    unsafe {
        let err = get_errno();
        let err_str = strerror(err);
        format!("{}: {} (errno: {})", context, CStr::from_ptr(err_str).to_string_lossy(), err)
    }
}

/// A semaphore implementation for inter-process synchronization.
#[derive(Debug)]
pub struct Semaphore {
    id: *mut libc::sem_t,
    name: String,
    creator: bool,
}

impl Semaphore {
    /// Creates a new named semaphore with the given initial value.
    ///
    /// # Arguments
    /// * `name` - The name of the semaphore.
    /// * `initial_value` - The initial count of the semaphore.
    ///
    /// # Returns
    /// * `Ok(Self)` if the semaphore is created successfully.
    /// * `Err(String)` if the creation fails.
    pub fn create(name: &str, initial_value: u32) -> Result<Self, String> {
        let name_cstr = CString::new(name).map_err(|_| "Invalid semaphore name".to_string())?;
        unsafe { sem_unlink(name_cstr.as_ptr()) }; // Remove existing semaphore
        let id = unsafe { sem_open(name_cstr.as_ptr(), O_CREAT, (S_IRUSR | S_IWUSR) as c_int, initial_value as c_uint) };

        if id == SEM_FAILED {
            return Err(get_last_error(&format!("Failed to create semaphore {}", name)));
        }

        Ok(Self {
            id,
            name: name.to_string(),
            creator: true,
        })
    }

    /// Opens an existing named semaphore.
    ///
    /// # Arguments
    /// * `name` - The name of the semaphore to open.
    ///
    /// # Returns
    /// * `Ok(Self)` if the semaphore is opened successfully.
    /// * `Err(String)` if the operation fails.
    pub fn open(name: &str) -> Result<Self, String> {
        let name_cstr = CString::new(name).map_err(|_| "Invalid semaphore name".to_string())?;
        let id = unsafe { sem_open(name_cstr.as_ptr(), 0) };

        if id == SEM_FAILED {
            return Err(get_last_error(&format!("Failed to open semaphore {}", name)));
        }

        Ok(Self {
            id,
            name: name.to_string(),
            creator: false,
        })
    }

    /// Performs a blocking wait (decrement) operation on the semaphore.
    ///
    /// # Returns
    /// * `Ok(())` if successful.
    /// * `Err(String)` if the operation fails.
    pub fn wait(&self) -> Result<(), String> {
        if unsafe { sem_wait(self.id) } == -1 {
            return Err(get_last_error(&format!("Failed to lock semaphore {}", self.name)));
        }
        Ok(())
    }

    /// Attempts to perform a non-blocking wait (decrement) operation on the semaphore.
    ///
    /// # Returns
    /// * `Ok(true)` if the operation succeeds.
    /// * `Ok(false)` if the semaphore is unavailable.
    /// * `Err(String)` if an error occurs.
    pub fn try_wait(&self) -> Result<bool, String> {
        if unsafe { sem_trywait(self.id) } == -1 {
            let err = unsafe { get_errno() };
            if err == libc::EAGAIN {
                // The  operation  could  not  be  performed without blocking (i.e., the semaphore currently has the value zero).
                return Ok(false);
            }
            return Err(get_last_error(&format!("Failed to try-lock semaphore {}", self.name)));
        }
        Ok(true)
    }

    /// Performs a post (increment) operation on the semaphore.
    ///
    /// # Returns
    /// * `Ok(())` if successful.
    /// * `Err(String)` if the operation fails.
    pub fn post(&self) -> Result<(), String> {
        if unsafe { sem_post(self.id) } == -1 {
            return Err(get_last_error(&format!("Failed to unlock semaphore {}", self.name)));
        }
        Ok(())
    }

    /// Retrieves the current value of the semaphore (Linux only).
    ///
    /// # Returns
    /// * `Ok(u32)` representing the semaphore value.
    /// * `Err(String)` if the operation fails.
    #[cfg(target_os = "linux")]
    pub fn get_value(&self) -> Result<u32, String> {
        let mut value: c_int = 0;
        if unsafe { libc::sem_getvalue(self.id, &mut value) } == -1 {
            return Err(get_last_error(&format!("Failed to get semaphore value {}", self.name)));
        }
        Ok(value as u32)
    }
    #[cfg(target_os = "macos")]
    pub fn get_value(&self) -> Result<u32, String> {
        Ok(0)
    }
}

impl Drop for Semaphore {
    /// Closes and optionally removes the semaphore when dropped.
    fn drop(&mut self) {
        unsafe {
            if sem_close(self.id) == -1 {
                let err = get_errno();
                eprintln!("Warning: sem_close failed {}: {}", self.name, err);
            }

            if self.creator {
                let name_cstr = CString::new(self.name.clone()).expect("Failed to create CString");
                if sem_unlink(name_cstr.as_ptr()) == -1 {
                    let err = get_errno();
                    eprintln!("Warning: sem_unlink failed {}: {}", self.name, err);
                }
            }
        }
    }
}

/// A shared memory segment with reader-writer locking.
///
/// This structure allows multiple readers and exclusive writers to access
/// a shared memory segment safely using semaphores for synchronization.
///
/// # Safety
/// This struct is manually marked as `Send` and `Sync` because it ensures
/// proper synchronization mechanisms are in place to allow safe concurrent
/// access.
pub struct RWLockedSharedMemory {
    mmap_ptr: *mut u8,
    write_lock: Semaphore,
    reader_count: Semaphore,
    mmap_path: String,
    is_creator: bool,
    size: usize,
}
unsafe impl Send for RWLockedSharedMemory {}
unsafe impl Sync for RWLockedSharedMemory {}

impl RWLockedSharedMemory {
    /// Creates a new shared memory segment with reader-writer locking.
    ///
    /// # Arguments
    /// * `mmap_path` - The file path for the shared memory.
    /// * `size` - The size of the shared memory.
    ///
    /// # Returns
    /// * `Ok(Self)` on success.
    /// * `Err(String)` on failure.
    pub fn create(mmap_path: &str, size: usize) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o777)
            .open(mmap_path)
            .map_err(|e| format!("Unable to create shared memory file {}: {}", mmap_path, e))?;

        file.set_len(size as u64).map_err(|e| format!("Unable to set file size {}: {}", mmap_path, e))?;

        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_SHARED, file.as_raw_fd(), 0) };

        if addr == MAP_FAILED {
            return Err(get_last_error(&format!("Failed to map memory {}", mmap_path)));
        }

        let write_lock_name = format!("/{}_protect_write", mmap_path.replace("/", "_"));
        let read_count_name = format!("/{}_read_count_write", mmap_path.replace("/", "_"));

        let write_lock = Semaphore::create(&write_lock_name, 1)?;
        let read_count = Semaphore::create(&read_count_name, 0)?;

        Ok(Self {
            mmap_ptr: addr as *mut u8,

            write_lock,
            reader_count: read_count,

            mmap_path: mmap_path.to_string(),
            is_creator: true,
            size,
        })
    }

    /// Opens an existing shared memory segment with reader-writer locking.
    ///
    /// # Arguments
    /// * `mmap_path` - The file path for the shared memory.
    /// * `size` - The size of the shared memory.
    ///
    /// # Returns
    /// * `Ok(Self)` on success.
    /// * `Err(String)` on failure.
    pub fn open(mmap_path: &str, size: usize) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(mmap_path)
            .map_err(|e| format!("Unable to open shared memory file {}: {}", mmap_path, e))?;

        let addr = unsafe { mmap(ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_SHARED, file.as_raw_fd(), 0) };

        if addr == MAP_FAILED {
            return Err(get_last_error(&format!("Failed to map memory {}", mmap_path)));
        }

        let write_lock_name = format!("/{}_protect_write", mmap_path.replace("/", "_"));
        let read_count_name = format!("/{}_read_count_write", mmap_path.replace("/", "_"));

        let write_lock = Semaphore::open(&write_lock_name)?;
        let read_count = Semaphore::open(&read_count_name)?;

        Ok(Self {
            mmap_ptr: addr as *mut u8,

            write_lock: write_lock,
            reader_count: read_count,

            mmap_path: mmap_path.to_string(),
            is_creator: false,
            size,
        })
    }

    /// Writes serialized data to shared memory with writer synchronization.
    ///
    /// # Arguments
    /// * `data` - The data to serialize and write.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    /// * `Err(String)` on failure.
    pub fn write<T>(&self, data: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let encoded: Vec<u8> = bincode::serialize(data).map_err(|e| format!("Serialization error: {}", e))?;
        let length_bytes = encoded.len().to_ne_bytes();

        self.write_lock.wait()?; // now i have the permission to write, other readers and writers are blocked, but readers can be still active

        // test if there are still readers active
        'x: loop {
            match self.reader_count.try_wait() {
                Ok(false) => {
                    // We have no active readers
                    break 'x;
                }
                Ok(true) => {
                    // there is at least one reader active
                    // correct the read-count (try_wait has decremented it)
                    self.reader_count.post()?;
                    thread::sleep(Duration::from_millis(30)); //wait till next try
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        unsafe {
            ptr::write(self.mmap_ptr as *mut i8, 0);
            ptr::copy_nonoverlapping(length_bytes.as_ptr(), self.mmap_ptr.add(1), length_bytes.len());
            ptr::copy_nonoverlapping(encoded.as_ptr(), self.mmap_ptr.add(1 + length_bytes.len()), encoded.len());
        }

        self.write_lock.post()?; // I'm ready

        Ok(())
    }

    /// Reads and deserializes data from shared memory with reader synchronization.
    ///
    /// # Returns
    /// * `Ok(Some(T))` if data is successfully read and deserialized.
    /// * `Ok(None)` if no valid data is found.
    /// * `Err(String)` if an error occurs.
    pub fn read<T>(&self) -> Result<Option<T>, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.write_lock.wait()?; // are there active writers

        match self.reader_count.try_wait() {
            Ok(false) => {
                // we are the first reader
            }
            Ok(true) => {
                // we are not the first reader
                self.reader_count.post()?; // correct the read-count, try_wait has decremented it
            }
            Err(err) => {
                return Err(err);
            }
        }
        self.reader_count.post()?; // increment the read count, we are a new reader

        // give others readers a chance to read
        // now writers are also allowed, but they check the read_count
        self.write_lock.post()?;

        // now, we can read
        let result = unsafe {
            if ptr::read(self.mmap_ptr as *const i8) == -1 {
                None
            } else {
                let mut length_bytes = [0u8; std::mem::size_of::<usize>()];
                ptr::copy_nonoverlapping(self.mmap_ptr.add(1), length_bytes.as_mut_ptr(), length_bytes.len());
                let data_len = usize::from_ne_bytes(length_bytes);
                let mut buffer = vec![0u8; data_len];
                ptr::copy_nonoverlapping(self.mmap_ptr.add(1 + length_bytes.len()), buffer.as_mut_ptr(), data_len);
                bincode::deserialize(&buffer).ok()
            }
        };

        self.reader_count.wait()?; // decrement read-count, this can never block, since we are here

        // test if we are the last reader
        match self.reader_count.try_wait() {
            Ok(false) => {
                // we are the last reader
            }
            Ok(true) => {
                // we are not the last reader
                self.reader_count.post()?; // correct the read count value
            }
            Err(err) => {
                return Err(err);
            }
        }
        Ok(result)
    }
}

impl Drop for RWLockedSharedMemory {
    fn drop(&mut self) {
        unsafe {
            ptr::write(self.mmap_ptr as *mut i8, -1);
            if munmap(self.mmap_ptr as *mut _, self.size) == -1 {
                let err = get_errno();
                eprintln!("Warning: munmap failed {}: {}", self.mmap_path, err);
            }

            if self.is_creator {
                if let Err(e) = remove_file(&self.mmap_path) {
                    eprintln!("Warning: remove failed {}: {}", self.mmap_path, e);
                }
            }
        }
    }
}
