use super::{as_from_bytes::AsFromBytes, shm_mapping::ShmMapping};
use crate::graph_structure::graph::DirectedAcyclicGraph;
use anyhow::{anyhow, Result};
use std::sync::atomic::AtomicU8;

impl ShmMapping<DirectedAcyclicGraph> {
    /// Execute graph stored in shared memory mapping.
    pub fn execute_graph(&mut self) -> Result<()> {
        // TODO: move code from `graph.rs` here.
        self.wrapped.execute_nodes()?;
        Ok(())
    }

    // /// Update node execution status of graph stored in shared memory.
    // pub fn update_node_execution_status(&mut self, node: NodeIndex, execution_status: ExecutionStatus) -> Result<()> {
    //     // Update `graph` field in `ShmMemory` strct.
    //     self.graph[node].execution_status = execution_status;

    //     // Update graph in shared memory mapping.
    //     let (is_init, raw_ptr) = self.open_shm_mapping()?;
    //     self.write_graph_from_struct_to_memory(is_init, raw_ptr)?;

    //     Ok(())
    // }

    /// Read graph bytes from shared memory.
    pub fn read_from_shared_memory(&mut self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let mutex = ShmMapping::<DirectedAcyclicGraph>::get_mutex(is_init, raw_ptr)?;
        let guard = mutex.lock().map_err(|e| anyhow!("Error acquiring Mutex lock: {}", e))?;
        // let val: &mut u8 = unsafe { &mut **guard };

        // Read from shared memory mapping.
        let mut graph_bytes: Vec<u8> = vec![];
        unsafe {
            for _ in 0..self.buf_len {
                graph_bytes.push(raw_ptr.read());
                raw_ptr = raw_ptr.add(1);
            }
        }

        // Release lock over guarded data.
        drop(guard);

        self.wrapped = if self.serialize {
            rmp_serde::from_slice::<DirectedAcyclicGraph>(&graph_bytes)?
        } else {
            DirectedAcyclicGraph::from_bytes(&graph_bytes)
        };
        Ok(())
    }
}
