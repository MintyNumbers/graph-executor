use crate::{graph_structure::execution_status::ExecutionStatus, graph_structure::graph::DirectedAcyclicGraph, shared_memory::shm_mapping::ShmMapping};
use anyhow::{anyhow, Result};
use iceoryx2_cal::dynamic_storage::{posix_shared_memory::Storage, DynamicStorage};
use petgraph::Direction;
use rand::Rng;
use std::{sync::atomic::AtomicU8, thread, time::Duration};

/// Execute graph stored in shared memory mapping.
pub fn execute_graph(filename_prefix: String, dag: DirectedAcyclicGraph) -> Result<()> {
    // Create/open shared memory mapping for `graph`.
    let mut shm_graph = match ShmMapping::<Storage<AtomicU8>>::new(&filename_prefix, &dag) {
        Ok(shm_graph) => shm_graph,
        Err(e) => {
            if e.to_string()
                == format!(
                    "Failed to create write_lock: Failed to create semaphore /{}_write_lock: File exists (errno: 17)",
                    &filename_prefix
                )
            {
                ShmMapping::<Storage<AtomicU8>>::open::<DirectedAcyclicGraph>(&filename_prefix)?.0
            } else {
                Err(anyhow!("Failed to create shared memory {}: {}", &filename_prefix, e))?
            }
        }
    };

    let mut rng = rand::rng();
    loop {
        // Get an executable `Node`, set `execution_status` for `node_index` to `ExecutionStatus::Executing` and execute associated `Node`.
        // If no executable `Node` is available or the chosen `Node` is already being executed by another process sleep for 10ms.
        let mut dag_in_shm = shm_graph.read::<DirectedAcyclicGraph>()?;
        let mut dag;
        let node_index = 'x: loop {
            // Try to execute an `Executable` `Node`
            if let Some(i) = dag_in_shm.get_executable_node_index() {
                dag = dag_in_shm.clone();
                dag[i].execution_status = ExecutionStatus::Executing;
                match shm_graph.shm_compare_graph_and_swap::<DirectedAcyclicGraph>(&dag_in_shm, &dag)? {
                    Some(new_dag_in_shm) => dag_in_shm = new_dag_in_shm, // Update `dag_in_shm` representation if the graph in shared memory was changed in the meantime
                    None => break 'x i, // Return current graph and `NodeIndex` if no process has already started executing associated `Node` in the meantime
                }
            }
            // End loop if graph is executed
            else if dag_in_shm.is_graph_executed() {
                return Ok(());
            }
            // Update `dag_in_shm`
            else {
                thread::sleep(Duration::from_millis(rng.random_range(10..100))); // Sleep if no `Executable` `Node` is available
                dag_in_shm = shm_graph.read()?;
            }
        };
        dag[node_index].execute()?;
        thread::sleep(Duration::from_secs(1));

        // Set `execution_status` for `node_index` to `ExecutionStatus::Executed`.
        // Get indeces of `Node`s that are now executable (due to all their parent nodes having been executed).
        dag_in_shm = shm_graph.read::<DirectedAcyclicGraph>()?;
        'x: loop {
            dag = dag_in_shm.clone();
            dag[node_index].execution_status = ExecutionStatus::Executed;
            let dag_temp = dag.clone();
            // Iterate through all child nodes (`child_index`) of `node_index`
            for child_index in dag_temp.graph.neighbors_directed(node_index, Direction::Outgoing) {
                // If all parent nodes (`parent_index`) of `child_index` are executed, then `child_index` is executable
                if dag_temp
                    .graph
                    .neighbors_directed(child_index, Direction::Incoming)
                    .all(|parent_index| dag_temp.graph[parent_index].execution_status == ExecutionStatus::Executed)
                {
                    dag[child_index].execution_status = ExecutionStatus::Executable;
                }
            }
            // Write graph to shared memory
            match shm_graph.shm_compare_graph_and_swap(&dag_in_shm, &dag)? {
                Some(new_dag_in_shm) => dag_in_shm = new_dag_in_shm,
                None => break 'x,
            }
        }
    }
}
