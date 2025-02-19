use crate::graph_structure::graph::DirectedAcyclicGraph;
use anyhow::{anyhow, Result};
use raw_sync::locks::*;
use shared_memory::ShmemConf;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Debug)]
pub struct ShmMapping {
    shmem_flink: String,
    buf_len: usize,
    graph: DirectedAcyclicGraph,
}

impl ShmMapping {
    /// Creates a new shared memory mapping, writes graph to it, initializes `Mutex` and returns `ShmMapping`.
    pub fn new(shmem_flink: String, graph: DirectedAcyclicGraph) -> Result<Self> {
        // Construct `ShmMapping`.
        let shm_mapping = ShmMapping {
            shmem_flink: shmem_flink,
            buf_len: rmp_serde::to_vec(&graph)?.len(),
            graph: graph,
        };

        // Create shared memory mapping.
        let _ = std::fs::remove_file(&shm_mapping.shmem_flink);
        let shmem = ShmemConf::new()
            .size(shm_mapping.buf_len)
            .flink(&shm_mapping.shmem_flink)
            .create()
            .map_err(|e| anyhow!("Unable to create new shared memory section {}: {}", &shm_mapping.shmem_flink, e))?;

        // Initialize `raw_ptr`.
        let mut raw_ptr: *mut u8 = shmem.as_ptr(); // Separate initialization required to prevent segmentation fault: core dump.
        let is_init: &mut AtomicU8;
        unsafe {
            is_init = &mut *(raw_ptr as *mut u8 as *mut AtomicU8);
            raw_ptr = raw_ptr.add(8);
        };

        // Initialize mutex.
        is_init.store(0, Ordering::Relaxed);
        unsafe {
            Mutex::new(
                raw_ptr,                                    // Base address of Mutex.
                raw_ptr.add(Mutex::size_of(Some(raw_ptr))), // Address of data protected by mutex.
            )
            .map_err(|e| anyhow!("Failed to initialize Mutex: {}", e))?
        };
        is_init.store(1, Ordering::Relaxed);

        // Write graph to shared memory mapping.
        shm_mapping.write_graph_from_struct_to_memory(is_init, raw_ptr)?;

        // Return `ShmMapping`.
        Ok(shm_mapping)
    }

    /// Update `self.graph` and the graph stored in the shared memory mapping with supplied `graph`.
    pub fn _update_graph(&mut self, graph: DirectedAcyclicGraph) -> Result<()> {
        // Update `graph` field in `ShmMemory` strct.
        self.graph = graph;

        // Update graph in shared memory mapping.
        let (is_init, raw_ptr) = self._open_shm_mapping()?;
        self.write_graph_from_struct_to_memory(is_init, raw_ptr)?;

        Ok(())
    }

    pub fn execute_graph(&mut self) -> Result<()> {
        self.graph.execute_nodes()?;
        Ok(())
    }

    /// Open shared memory mapping.
    fn _open_shm_mapping(&self) -> Result<(&mut AtomicU8, *mut u8)> {
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

    fn get_mutex(is_init: &mut AtomicU8, raw_ptr: *mut u8) -> Result<Box<dyn LockImpl>> {
        // Wait for initialized mutex.
        while is_init.load(Ordering::Relaxed) != 1 {
            println!("This shouldn't happen?");
        }

        // Load existing mutex.
        let (mutex, _bytes_used) = unsafe {
            Mutex::from_existing(
                raw_ptr,                                    // Base address of Mutex
                raw_ptr.add(Mutex::size_of(Some(raw_ptr))), // Address of data  protected by mutex
            )
            .map_err(|e| anyhow!("Error loading Mutex: {}", e))?
        };

        Ok(mutex)
    }

    /// Write serialized graph to shared memory.
    fn write_graph_from_struct_to_memory(&self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let mutex = ShmMapping::get_mutex(is_init, raw_ptr)?;
        let _ = mutex.lock().map_err(|e| anyhow!("Error acquiring Mutex lock: {}", e))?;
        // let val: &mut u8 = unsafe { &mut **guard };

        // Write to shared memory mapping.
        unsafe {
            raw_ptr = raw_ptr.add(Mutex::size_of(Some(raw_ptr))); // Move pointer to data address.
            let serialized_graph: Vec<u8> = rmp_serde::to_vec(&self.graph)?;
            for byte in serialized_graph {
                raw_ptr.write(byte); // Write serialized byte to memory.
                raw_ptr = raw_ptr.add(16); // Move pointer forward by a byte.
            }
        }

        Ok(())
        // Release lock over guarded data.
    }

    /// Read serialized graph from shared memory.
    pub fn _read_graph_from_memory_to_struct(&mut self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let mutex = ShmMapping::get_mutex(is_init, raw_ptr)?;
        let _ = mutex.lock().map_err(|e| anyhow!("Error acquiring Mutex lock: {}", e))?;
        // let val: &mut u8 = unsafe { &mut **guard };

        // Read from shared memory mapping.
        let mut serialized_graph: Vec<u8> = vec![];
        unsafe {
            for _ in 0..self.buf_len {
                serialized_graph.push(raw_ptr.read());
                raw_ptr = raw_ptr.add(16);
            }
        }

        let graph: DirectedAcyclicGraph = rmp_serde::from_slice(&serialized_graph)?;
        self.graph = graph;
        Ok(())
        // Release lock over guarded data.
    }
}
