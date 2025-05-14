use super::semaphore::Semaphore;

use anyhow::{anyhow, Result};
use std::{thread, time::Duration};

pub(crate) fn read_lock(write_lock: &Semaphore, read_count: &Semaphore) -> Result<()> {
    write_lock.wait().map_err(|e| anyhow!("Failed blocking (decrementing) write_lock: {}", e))?; // are there active writers
    match read_count.try_wait() {
        Ok(false) => (), // we are the first reader
        Ok(true) => {
            // we are not the first reader
            // correct the read-count, try_wait has decremented it
            read_count.post().map_err(|e| anyhow!("Failed incrementing read_lock semaphore: {}", e))?;
        }
        Err(e) => return Err(anyhow!("Failed to acquire read_lock: {}", e)),
    }
    read_count.post().map_err(|e| anyhow!("Failed incrementing read_lock semaphore: {}", e))?; // increment the read count, we are a new reader

    // give others readers a chance to read
    // now writers are also allowed, but they check the read_count
    write_lock.post().map_err(|e| anyhow!("Failed icrementing write_lock semaphore: {}", e))?;

    Ok(())
}

pub(crate) fn read_unlock(read_count: &Semaphore) -> Result<()> {
    read_count.wait().map_err(|e| anyhow!("{}", e))?; // decrement read-count, this can never block, since we are here

    // test if we are the last reader
    match read_count.try_wait() {
        Ok(false) => {
            // we are the last reader
        }
        Ok(true) => {
            // we are not the last reader
            // correct the read count value
            read_count.post().map_err(|e| anyhow!("Failed incrementing read_lock: {}", e))?;
        }
        Err(e) => {
            return Err(anyhow!("Failed to perform non-blocking wait (decrement) on read_lock semaphore: {}", e));
        }
    }

    Ok(())
}

pub(crate) fn write_lock(write_lock: &Semaphore, read_count: &Semaphore) -> Result<()> {
    write_lock.wait().map_err(|e| anyhow!("Failed acquiring lock: {}", e))?; // Now I have the permission to write, other readers and writers are blocked, but readers can be still active

    // Test if there are still readers active
    'x: loop {
        match read_count.try_wait() {
            Ok(false) => break 'x, // We have no active readers
            Ok(true) => {
                // There is at least one reader active
                // Correct the read-count (try_wait has decremented it)
                read_count.post().map_err(|e| anyhow!("Failed posting read_count Semaphore: {}", e))?;
                thread::sleep(Duration::from_millis(30)); // wait until next try
            }
            Err(e) => return Err(anyhow!("Failed reading {}", e)),
        }
    }

    Ok(())
}

pub(crate) fn write_unlock(write_lock: &Semaphore) -> Result<()> {
    write_lock.post().map_err(|e| anyhow!("Failed posting write_lock Semaphore: {}", e))?;
    Ok(())
}
