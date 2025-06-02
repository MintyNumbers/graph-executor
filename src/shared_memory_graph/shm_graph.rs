use crate::{
    graph_structure::{execution_status::ExecutionStatus, graph::DirectedAcyclicGraph},
    shared_memory::posix_shared_memory::PosixSharedMemory,
};
use anyhow::{anyhow, Result};
use iceoryx2_cal::dynamic_storage::DynamicStorage;
use petgraph::graph::NodeIndex;
use std::sync::atomic::AtomicU8;

impl<S: DynamicStorage<AtomicU8>> PosixSharedMemory<S> {
    /// Acquire write lock and advance execution status to
    pub fn shm_compare_node_execution_status_and_update(
        &mut self,
        node_index: NodeIndex,
        new_execution_status: ExecutionStatus,
    ) -> Result<Option<DirectedAcyclicGraph>> {
        // Old execution status for conditional write
        let old_execution_status = match new_execution_status {
            ExecutionStatus::NonExecutable => return Err(anyhow!("New execution status cannot be ExecutionStatus::NonExecutable.")),
            ExecutionStatus::Executable => ExecutionStatus::NonExecutable,
            ExecutionStatus::Executing => ExecutionStatus::Executable,
            ExecutionStatus::Executed => ExecutionStatus::Executing,
        };

        // Acquire exclusive (write) lock
        self.write_lock()?;

        // Write data to shared memory if `data_condition` is equal to current state of data in shared memory
        let graph_bytes = self.read_from_shm()?;
        let mut graph_in_shm = rmp_serde::from_slice::<DirectedAcyclicGraph>(graph_bytes.as_slice())?;
        match graph_in_shm[node_index].execution_status == old_execution_status {
            true => {
                // Release write lock and return None on successful write
                graph_in_shm[node_index].execution_status = new_execution_status;
                self.write_to_shm(&graph_in_shm)?;
                self.write_unlock()?;
                return Ok(None);
            }
            false => {
                // Release write lock and if `data_condition` no longer matches return `data_in_shm`
                self.write_unlock()?;
                return Ok(Some(graph_in_shm));
            }
        }
    }
}
