use crate::graph_structure::{execution_status::ExecutionStatus, graph::DirectedAcyclicGraph};
use crate::shared_memory::posix_shared_memory::PosixSharedMemory;
use anyhow::{anyhow, Result};
use iceoryx2_cal::dynamic_storage::DynamicStorage;
use petgraph::{graph::NodeIndex, Direction};
use rand::Rng;
use std::{collections::VecDeque, sync::atomic::AtomicU8, thread, time::Duration};

impl DirectedAcyclicGraph {
    /// Execute graph stored in shared memory mapping.
    pub fn execute_graph<S: DynamicStorage<AtomicU8>>(&mut self, filename_prefix: String) -> Result<()> {
        // Create/open shared memory mapping for `graph`.
        let mut shared_memory = match PosixSharedMemory::<S>::new(&filename_prefix, &self) {
            Ok(shared_memory) => shared_memory,
            Err(e) => {
                if e.to_string()
                    == format!(
                        "Failed to create write_lock: Failed to create semaphore /{}_write_lock: File exists (errno: 17)",
                        &filename_prefix
                    )
                {
                    PosixSharedMemory::<S>::open::<DirectedAcyclicGraph>(&filename_prefix)?.0
                } else {
                    Err(anyhow!("Failed to create shared memory {}: {}", &filename_prefix, e))?
                }
            }
        };

        let mut rng = rand::rng();
        loop {
            // Get an executable `Node`, set `execution_status` for `node_index` to `ExecutionStatus::Executing` and execute associated `Node`.
            // If no executable `Node` is available or the chosen `Node` is already being executed by another process sleep for 10ms.
            *self = shared_memory.read::<DirectedAcyclicGraph>()?;
            let node_index = 'x: loop {
                // Try to execute an `Executable` `Node`
                if let Some(i) = self.get_executable_node_index() {
                    match shared_memory.shm_compare_node_execution_status_and_update(i, ExecutionStatus::Executing)? {
                        Some(new_dag_in_shm) => *self = new_dag_in_shm, // Update `dag_in_shm` representation if the graph in shared memory was changed in the meantime
                        None => break 'x i, // Return current graph and `NodeIndex` if no process has already started executing associated `Node` in the meantime
                    }
                }
                // End loop if graph is executed
                else if self.is_graph_executed() {
                    return Ok(());
                }
                // Update `dag_in_shm`
                else {
                    thread::sleep(Duration::from_millis(rng.random_range(10..100))); // Sleep if no executable `Node` is available
                    *self = shared_memory.read()?;
                }
            };
            self[node_index].execution_status = ExecutionStatus::Executing;
            self[node_index].execute()?;

            // Set `execution_status` for `node_index` to `ExecutionStatus::Executed`.
            self[node_index].execution_status = ExecutionStatus::Executed;
            if let Some(new_dag_in_shm) = shared_memory.shm_compare_node_execution_status_and_update(node_index, ExecutionStatus::Executed)? {
                // If a `DirectedAcyclicGraph` is returned, then the `node_index`' `execution_status` was changed by another process.
                return Err(anyhow!(
                    "Execution status of {:?} changed: {} by another process.",
                    node_index,
                    new_dag_in_shm[node_index]
                ));
            };

            // Get indeces of `Node`s that are now executable (due to all their parent nodes having been executed).
            let mut children_indeces: VecDeque<NodeIndex> = self.graph.neighbors_directed(node_index, Direction::Outgoing).collect();
            // Iterate through all child nodes of `node_index`.
            while children_indeces.len() > 0 {
                // Get first `child_index` from queue.
                let child_index = children_indeces
                    .pop_front()
                    .ok_or(anyhow!("No child index despite queue having more than 0 elements"))?;

                // Read graph from shared memory to learn newest execution statuses.
                *self = shared_memory.read::<DirectedAcyclicGraph>()?;

                // Determine whether all parent nodes `p` of child node are executed or executing
                let (all_executed, all_executed_or_executing) = {
                    let (mut all_executed, mut all_executed_or_executing) = (true, true);
                    for p in self.graph.neighbors_directed(child_index, Direction::Incoming) {
                        // If some node is executing, then not all parent nodes are executed
                        if self[p].execution_status == ExecutionStatus::Executing {
                            all_executed = false;
                        }
                        // If some node is neither executed nor executing, then not all parent nodes are executed or executing
                        else if self[p].execution_status != ExecutionStatus::Executed && self[p].execution_status != ExecutionStatus::Executing {
                            (all_executed, all_executed_or_executing) = (false, false);
                            break;
                        }
                    }
                    (all_executed, all_executed_or_executing)
                };

                // If all parent nodes (`parent_index`) of `child_index` are executed, then `child_index` is executable.
                if all_executed {
                    // Write execution status to shared memory.
                    // Return value must be written immediately back to `current_graph`, because child node may be a parent of another child node.
                    if let Some(new_dag_in_shm) = shared_memory.shm_compare_node_execution_status_and_update(child_index, ExecutionStatus::Executable)? {
                        self[child_index].execution_status = new_dag_in_shm[child_index].execution_status;
                    } else {
                        self[child_index].execution_status = ExecutionStatus::Executable;
                    }
                } else if all_executed_or_executing {
                    // Keep child index in queue to check parent execution status later to make sure node is set to executable.
                    children_indeces.push_back(child_index);
                }
            }
        }
    }
}
