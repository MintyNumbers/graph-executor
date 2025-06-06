use super::semaphore::Semaphore;
use anyhow::{anyhow, Result};
use std::{thread, time::Duration};

/// Acquire read lock by:
/// - Decrement write_lock semaphore, thereby write locking and checking that there is no active writer
/// - Decrement read_count to check whether first reader and correcting read_count if necessary
/// - Register new reader by incrementing read_count semaphore
/// - Incrementing write_lock semaphore to unlock write_lock
pub(crate) fn read_lock(write_lock: &Semaphore, read_count: &Semaphore) -> Result<()> {
    // Check if there are active writers
    write_lock
        .wait()
        .map_err(|e| anyhow!("Failed locking write_lock semaphore: {}", e))?;

    // TODO: decide whether this block is necessary
    match read_count.try_wait() {
        Ok(false) => (), // First reader
        Ok(true) => {
            // Not the first reader
            // correct the read-count, try_wait has decremented it
            read_count.post().map_err(|e| {
                anyhow!(
                    "Failed correcting read_count semaphore after decrementing it: {}",
                    e
                )
            })?;
        }
        Err(e) => return Err(anyhow!("Failed decrementing read_count semaphore: {}", e)),
    }

    // Indicate presence of new reader
    read_count.post().map_err(|e| {
        anyhow!(
            "Failed incrementing read_count semaphore to indicate new active reader: {}",
            e
        )
    })?;

    // Allow new writers (which have to check read_count) and readers
    write_lock
        .post()
        .map_err(|e| anyhow!("Failed unlocking write_lock semaphore: {}", e))?;

    Ok(())
}

/// Release write lock by:
/// - Decrement read_count to unregister active reader.
pub(crate) fn read_unlock(read_count: &Semaphore) -> Result<()> {
    // Decrement read_count semaphore to unregister reader
    match read_count.try_wait() {
        Ok(false) => {
            return Err(anyhow!(
                "Decrementing read_count semaphore (unregistering a reader), which is equal to 0 and therefore indicating no active readers."
            ))
        }
        Ok(true) => (), // Successfully unregistered reader
        Err(e) => return Err(anyhow!("Failed decrementing read_count semaphore: {}", e)),
    }

    // TODO: decide whether this block is necessary
    match read_count.try_wait() {
        Ok(false) => (), // Last reader
        Ok(true) => {
            // we are not the last reader
            // correct the read count value
            read_count
                .post()
                .map_err(|e| anyhow!("Failed incrementing read_count: {}", e))?;
        }
        Err(e) => {
            return Err(anyhow!("Failed decrementing read_count: {}", e));
        }
    }

    Ok(())
}

/// Acquire write lock by:
/// - Decrement write_lock semaphore's value if it is greater than 0 (indicating there are current writers);
///   else block main thread until it is greater than 0 and decrement then.
/// - Wait until read_count semaphore's value is equal to 0, indicating there are no active readers anymore.
pub(crate) fn write_lock(write_lock: &Semaphore, read_count: &Semaphore) -> Result<()> {
    // Get writing permission, new readers and writers are blocked, but readers can be still active
    write_lock
        .wait()
        .map_err(|e| anyhow!("Failed acquiring lock: {}", e))?;

    // Test if there are still active readers
    'x: loop {
        match read_count.try_wait() {
            Ok(false) => break 'x, // No active readers
            Ok(true) => {
                // There is at least one reader active
                // Correct the read-count (try_wait has decremented it)
                read_count
                    .post()
                    .map_err(|e| anyhow!("Failed posting read_count Semaphore: {}", e))?;
                thread::sleep(Duration::from_millis(30)); // wait until next try
            }
            Err(e) => return Err(anyhow!("Failed reading {}", e)),
        }
    }

    Ok(())
}

/// Release write lock by:
/// - Increment write_lock semaphore value; a greater than 0 value indicates a writable state to other processes.
pub(crate) fn write_unlock(write_lock: &Semaphore) -> Result<()> {
    // TODO: decide if asserting no current readers is necessary (inner state validation check).

    write_lock
        .post()
        .map_err(|e| anyhow!("Failed posting write_lock Semaphore: {}", e))?;
    Ok(())
}
