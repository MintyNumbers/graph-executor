use crate::{
    graph_structure::execution_status::ExecutionStatus, graph_structure::graph::DirectedAcyclicGraph, shared_memory::posix_shared_memory::PosixSharedMemory,
};
use anyhow::{anyhow, Result};
use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
use petgraph::{graph::NodeIndex, Direction};
use rand::Rng;
use std::{sync::atomic::AtomicU8, thread, time::Duration};

/// Execute graph stored in shared memory mapping.
pub fn execute_graph(filename_prefix: String, initial_dag: DirectedAcyclicGraph) -> Result<()> {
    // Create/open shared memory mapping for `graph`.
    let mut shared_memory = match PosixSharedMemory::<Storage<AtomicU8>>::new(&filename_prefix, &initial_dag) {
        Ok(shared_memory) => shared_memory,
        Err(e) => {
            if e.to_string()
                == format!(
                    "Failed to create write_lock: Failed to create semaphore /{}_write_lock: File exists (errno: 17)",
                    &filename_prefix
                )
            {
                PosixSharedMemory::<Storage<AtomicU8>>::open::<DirectedAcyclicGraph>(&filename_prefix)?.0
            } else {
                Err(anyhow!("Failed to create shared memory {}: {}", &filename_prefix, e))?
            }
        }
    };

    let mut rng = rand::rng();
    loop {
        // Get an executable `Node`, set `execution_status` for `node_index` to `ExecutionStatus::Executing` and execute associated `Node`.
        // If no executable `Node` is available or the chosen `Node` is already being executed by another process sleep for 10ms.
        let mut current_dag = shared_memory.read::<DirectedAcyclicGraph>()?;
        let node_index = 'x: loop {
            // Try to execute an `Executable` `Node`
            if let Some(i) = current_dag.get_executable_node_index() {
                match shared_memory.shm_compare_node_execution_status_and_update(i, ExecutionStatus::Executing)? {
                    Some(new_dag_in_shm) => current_dag = new_dag_in_shm, // Update `dag_in_shm` representation if the graph in shared memory was changed in the meantime
                    None => break 'x i, // Return current graph and `NodeIndex` if no process has already started executing associated `Node` in the meantime
                }
            }
            // End loop if graph is executed
            else if current_dag.is_graph_executed() {
                return Ok(());
            }
            // Update `dag_in_shm`
            else {
                thread::sleep(Duration::from_millis(rng.random_range(10..100))); // Sleep if no `Executable` `Node` is available
                current_dag = shared_memory.read()?;
            }
        };
        current_dag[node_index].execution_status = ExecutionStatus::Executing;
        current_dag[node_index].execute()?;

        // Set `execution_status` for `node_index` to `ExecutionStatus::Executed`.
        current_dag[node_index].execution_status = ExecutionStatus::Executed;
        if let Some(new_dag_in_shm) = shared_memory.shm_compare_node_execution_status_and_update(node_index, ExecutionStatus::Executed)? {
            // If a `DirectedAcyclicGraph` is returned, then the `node_index`' `execution_status` was changed by another process.
            return Err(anyhow!(
                "Execution status of {:?} changed: {} by another process.",
                node_index,
                new_dag_in_shm[node_index]
            ));
        };

        // Get indeces of `Node`s that are now executable (due to all their parent nodes having been executed).
        let mut children_indeces: Vec<NodeIndex> = current_dag.graph.neighbors_directed(node_index, Direction::Outgoing).collect();
        // Iterate through all child nodes (`child_index`) of `node_index`
        'x: loop {
            let child_index = match children_indeces.pop() {
                Some(i) => i,
                // If all children's execution status has been updated in shared memory, break loop
                None => break 'x,
            };

            // If all parent nodes (`parent_index`) of `child_index` are executed, then `child_index` is executable
            if current_dag
                .graph
                .neighbors_directed(child_index, Direction::Incoming)
                .all(|parent_index| current_dag.graph[parent_index].execution_status == ExecutionStatus::Executed)
            {
                // Write graph to shared memory
                current_dag[child_index].execution_status = ExecutionStatus::Executable;
                if let Some(new_dag_in_shm) = shared_memory.shm_compare_node_execution_status_and_update(child_index, ExecutionStatus::Executable)? {
                    // Throw error if execution status was changed to anything but `ExecutionStatus::Executed`
                    // by some other process. `ExecutionStatus::Executed` is acceptable, as some other parent
                    // node might have executed in parallel and is also updating its children's indeces.
                    if new_dag_in_shm[child_index].execution_status != ExecutionStatus::Executed {
                        return Err(anyhow!(
                            "Execution status of {:?} changed: {} by another process.",
                            child_index,
                            new_dag_in_shm[child_index]
                        ));
                    }
                }
            }
        }

        for n in current_dag.graph.neighbors_directed(node_index, Direction::Outgoing) {
            println!("{:?}: {}", n, current_dag[n].execution_status);
        }
    }
}
