use super::{as_from_bytes::AsFromBytes, shm_mapping::ShmMapping};
use crate::graph_structure::{execution_status::ExecutionStatus, graph::DirectedAcyclicGraph};
use anyhow::{anyhow, Result};
use petgraph::{graph::NodeIndex, Direction};
use std::{
    sync::{atomic::AtomicU8, Arc, Condvar, Mutex, RwLock},
    thread,
};

impl ShmMapping<DirectedAcyclicGraph> {
    /// Execute graph stored in shared memory mapping.
    pub fn execute_graph(&mut self) -> Result<()> {
        // Get number of threads. If there are more available cores than executable nodes,
        // spawn a thread for each executable node, else spawn a thread for each core.
        let (num_cpu_cores, num_init_executable_nodes) = (num_cpus::get(), self.wrapped.get_executable_node_indeces().len());
        let _num_threads = num_cpu_cores.min(num_init_executable_nodes);

        // Create Mutex for `self` and all executable `Node`s to share execution data between threads.
        let executable_nodes_mutex = Arc::new(Mutex::new(self.wrapped.get_executable_node_indeces()));
        let notify_thread_condvar = Condvar::new(); // For notifying about new executable nodes or finished graph execution.
        let self_lock = Arc::new(RwLock::new(self));

        // Handle to main thread to park during node execution.
        let main_thread = thread::current();

        // Spawn threads.
        thread::scope(|s| -> Result<()> {
            // TODO: create mechanism which:
            //   (1) On program start only spawns as many threads as necessary (as many as there are initally executable nodes).
            //   (2) Spawns more threads when there are more executable nodes than active threads, but only ever as many as there are cores.
            //   (3) Puts surplus threads to sleep using a Condition Variable when there are more active threads than executable nodes.
            // Currently: Spawns a thread for each CPU core and execute nodes.
            for _ in 0..num_cpu_cores {
                s.spawn(|| -> Result<()> {
                    loop {
                        // Get an executable node and go to sleep if there are none.
                        let mut executable_nodes = executable_nodes_mutex.lock().unwrap();
                        let node_index = loop {
                            if let Some(i) = executable_nodes.pop_front() {
                                break i;
                            } else {
                                // Don't enter block if the graph is already executed (no notifiers are left).
                                if self_lock.read().unwrap().wrapped.is_graph_executed() == false {
                                    // Can potentially wait for a long time.
                                    executable_nodes = notify_thread_condvar.wait(executable_nodes).unwrap();
                                }
                                // Break loop (ending thread) when the whole graph has been executed and unpark main thread.
                                if self_lock.read().unwrap().wrapped.is_graph_executed() == true {
                                    main_thread.unpark();
                                    return Ok(());
                                }
                            }
                        };
                        drop(executable_nodes);

                        // Set execution status for `node_index` to `ExecutionStatus::Executing` for an executable node.
                        self_lock.write().unwrap().wrapped.graph[node_index].execution_status = ExecutionStatus::Executing;
                        println!("{:?}: Set execution status to executing.", node_index);

                        // Execute the thread's `Node`.
                        println!("{:?}: Executing node...", node_index);
                        self_lock.read().unwrap().wrapped.graph[node_index].execute()?;

                        // Set execution_status for `node_index` to `ExecutionStatus::Executed`.
                        self_lock.write().unwrap().wrapped.graph[node_index].execution_status = ExecutionStatus::Executed;
                        println!("{:?}: Set execution status to executed.", node_index);

                        // Get indeces of nodes that are now executable (due to all their parent nodes having been executed).
                        let self_data = self_lock.read().unwrap();
                        let new_executable_nodes: Vec<(NodeIndex, ExecutionStatus)> = self_data
                            .wrapped
                            .graph
                            .neighbors_directed(node_index, Direction::Outgoing)
                            .filter_map(|next_index| {
                                // Nodes that need to be executed prior to executing `next_index` (parent nodes).
                                for parent_index in self_data
                                    .wrapped
                                    .graph
                                    .neighbors_directed(next_index, Direction::Incoming)
                                    .collect::<Vec<NodeIndex>>()
                                {
                                    // If one parent node has not been executed, break loop because child is not executable.
                                    if self_data.wrapped.graph[parent_index].execution_status != ExecutionStatus::Executed {
                                        return None;
                                    }
                                }
                                return Some((next_index, ExecutionStatus::Executable));
                            })
                            .collect();
                        drop(self_data);

                        // Notify all threads if graph was executed.
                        if self_lock.read().unwrap().wrapped.is_graph_executed() == true {
                            notify_thread_condvar.notify_all();
                        }

                        // Notify a thread for each new executable node.
                        new_executable_nodes.iter().for_each(|(i, _)| {
                            executable_nodes_mutex.lock().unwrap().push_back(*i);
                            notify_thread_condvar.notify_one();
                        });
                    }
                });
            }

            // Park main thread during node execution
            thread::park();

            Ok(())
        })?;

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

    /// Read graph bytes from shared memory to struct.
    /// TODO: figure out visibility.
    fn read_from_shared_memory(&mut self, is_init: &mut AtomicU8, mut raw_ptr: *mut u8) -> Result<()> {
        // Acquire lock over guarded data.
        let read_lock = ShmMapping::<DirectedAcyclicGraph>::get_rwlock(is_init, raw_ptr)?;
        let guard = read_lock.rlock().map_err(|e| anyhow!("Error acquiring Mutex lock: {}", e))?;
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
