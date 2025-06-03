use libc::{c_int, c_uint, sem_close, sem_open, sem_post, sem_trywait, sem_unlink, sem_wait, strerror, O_CREAT, O_EXCL, SEM_FAILED, S_IRUSR, S_IWUSR};
use std::{ffi::CStr, ffi::CString};

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
        let id = unsafe { sem_open(name_cstr.as_ptr(), O_CREAT | O_EXCL, (S_IRUSR | S_IWUSR) as c_int, initial_value as c_uint) };

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

    pub fn name(&self) -> &str {
        &self.name
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
