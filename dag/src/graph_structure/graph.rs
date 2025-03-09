use super::{edge::Edge, execution_status::ExecutionStatus, node::Node};
use crate::shared_memory::as_from_bytes::AsFromBytes;
use anyhow::{anyhow, Error, Ok, Result};
use petgraph::{acyclic::Acyclic, dot, graph::NodeIndex, prelude::StableDiGraph, Direction};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    fs::write,
    ops::{Index, IndexMut},
    str::FromStr,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)] // Guarantee stable layout across executions
pub struct DirectedAcyclicGraph {
    graph: StableDiGraph<Node, i32>,
}

impl fmt::Display for DirectedAcyclicGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel]))
    }
}

impl FromStr for DirectedAcyclicGraph {
    type Err = Error;
    /// Parses `DirectedAcyclicGraph` from String.
    ///
    /// ```
    /// let graph = DirectedAcyclicGraph::from_str(read_to_string("resources/example.dot")?.as_str())?;
    /// ```
    fn from_str(dag_string: &str) -> Result<Self> {
        let mut nodes: Vec<(usize, Node)> = vec![];
        let mut edges: Vec<Edge> = vec![];

        if dag_string.trim().starts_with("digraph") {
            for line in dag_string.trim().split("\n") {
                let split_line = line.trim().split(" ").collect::<Vec<&str>>();

                // If a line looks like "0 [ label = "Node 0" ]" parse it as a `Node`.
                if split_line[0].trim().chars().all(|c| c.is_ascii_digit()) && split_line[1].trim() == "[" {
                    nodes.push((
                        split_line[0].trim().parse::<usize>()?,
                        Node::from_str(
                            *line
                                .split('\"')
                                .collect::<Vec<&str>>()
                                .get(1)
                                .ok_or(anyhow!("DirectedAcyclicGraph::from_str parsing error: No node label."))?,
                        )?,
                    ));
                }
                // If a line looks like "0 -> 1 [ ]" parse it as an `Edge`.
                else if split_line[0].trim().chars().all(|c| c.is_ascii_digit())
                    && split_line[1].trim() == "->"
                    && split_line[2].trim().chars().all(|c| c.is_ascii_digit())
                    && split_line[3].trim() == "["
                {
                    edges.push(Edge::new((split_line[0].trim().parse::<usize>()?, split_line[2].trim().parse::<usize>()?)));
                }
            }
        }

        DirectedAcyclicGraph::new(nodes, edges)
    }
}

impl AsFromBytes for DirectedAcyclicGraph {}

impl Index<NodeIndex> for DirectedAcyclicGraph {
    type Output = Node;
    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.graph[index]
    }
}

impl IndexMut<NodeIndex> for DirectedAcyclicGraph {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.graph[index]
    }
}

impl PartialEq for DirectedAcyclicGraph {
    fn eq(&self, other: &Self) -> bool {
        if self.graph.node_indices().count() != other.graph.node_indices().count() || self.graph.edge_indices().count() != other.graph.edge_indices().count() {
            return false;
        }
        for n in self.graph.node_indices() {
            if self[n] != other[n] {
                return false;
            }
        }
        for e in self.graph.edge_indices() {
            if self.graph.edge_endpoints(e).unwrap() != other.graph.edge_endpoints(e).unwrap() {
                return false;
            }
        }
        true
    }
}

impl DirectedAcyclicGraph {
    /// Creates `DirectedAcyclicGraph` from `Vec<Node>` and `Vec<Edge>`.
    ///
    /// You can create a `DirectedAcyclicGraph` like this:
    /// ```
    /// let graph = DirectedAcyclicGraph::new(
    ///     vec![Node::new(), Node::new(), Node::new(), Node::new()],
    ///     vec![Edge::new((0, 1)), Edge::new((1, 2)), Edge::new((2, 3)), Edge::new((1, 3))],
    /// )?;
    /// ```
    pub fn new(nodes: Vec<(usize, Node)>, edges: Vec<Edge>) -> Result<Self> {
        let mut graph = StableDiGraph::<Node, i32>::new();
        let mut node_indeces = HashMap::new();

        // Populate graph with nodes.
        nodes.into_iter().for_each(|(i, node)| {
            node_indeces.insert(i, graph.add_node(node));
        });

        // Populate graph with all edges between nodes.
        edges.into_iter().for_each(|edge| {
            if edge.nodes.0 < node_indeces.len() && edge.nodes.1 < node_indeces.len() {
                graph.add_edge(node_indeces[&edge.nodes.0], node_indeces[&edge.nodes.1], 1);

                // Set `ExecutionStatus` of `edge.nodes.1` to `NonExecutable`.
                graph[node_indeces[&edge.nodes.1]].execution_status = ExecutionStatus::NonExecutable;
            }
        });

        // Check that `StableDiGraph` is acyclic and return `DirectedAcyclicGraph` if successful.
        Acyclic::try_from_graph(&graph).map_err(|e| anyhow!("Cyclic graph supplied on {:?}", e.node_id()))?;
        Ok(DirectedAcyclicGraph { graph: graph })
    }

    /// Write `DirectedAcyclicGraph` to `path`.
    ///
    /// ```
    /// let graph = DirectedAcyclicGraph::new(
    ///     vec![Node::new(), Node::new(), Node::new(), Node::new()],
    ///     vec![Edge::new((0, 1)), Edge::new((1, 2)), Edge::new((2, 3)), Edge::new((1, 3))],
    /// )?;
    /// graph.write_to_path("resources/example.dot")?;
    /// ```
    pub fn write_to_path(&self, path: &str) -> Result<()> {
        write(path, &format!("{}", dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])))?;
        Ok(())
    }

    /// Get all executable `Node` indeces.
    pub fn get_executable_node_indeces(&self) -> VecDeque<NodeIndex> {
        self.graph
            .node_indices()
            .filter_map(|i| {
                if self.graph[i].execution_status == ExecutionStatus::Executable {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get an executable `Node` index.
    pub fn get_executable_node_index(&self) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|i| self.graph[*i].execution_status == ExecutionStatus::Executable)
    }

    pub fn is_graph_executed(&self) -> bool {
        self.graph
            .node_weights()
            .filter_map(|n| if n.execution_status == ExecutionStatus::Executed { None } else { Some(n) })
            .collect::<Vec<&Node>>()
            .is_empty()
    }

    /// Execute all `Node`s.
    pub fn execute_nodes(&mut self) -> Result<()> {
        // Get number of threads. If more cores than executable nodes, spawn a thread for each executable node, else spawn a thread for each core.
        let (num_cpu_cores, _num_init_executable_nodes) = (num_cpus::get(), self.get_executable_node_indeces().len());
        let _num_threads = if num_cpu_cores > _num_init_executable_nodes {
            _num_init_executable_nodes
        } else {
            num_cpu_cores
        };

        // Create Mutex for `self` and all executable `Node`s to share execution data between threads.
        let executable_nodes_mutex = Arc::new(Mutex::new(self.get_executable_node_indeces()));
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
                                if self_lock.read().unwrap().is_graph_executed() == false {
                                    // Can potentially wait for a long time.
                                    executable_nodes = notify_thread_condvar.wait(executable_nodes).unwrap();
                                }
                                // Break loop (ending thread) when the whole graph has been executed and unpark main thread.
                                if self_lock.read().unwrap().is_graph_executed() == true {
                                    main_thread.unpark();
                                    return Ok(());
                                }
                            }
                        };
                        drop(executable_nodes);

                        // Set execution status for `node_index` to `ExecutionStatus::Executing` for an executable node.
                        self_lock.write().unwrap().graph[node_index].execution_status = ExecutionStatus::Executing;
                        println!("{:?}: Set execution status to executing.", node_index);

                        // Execute the thread's `Node`.
                        println!("{:?}: Executing node...", node_index);
                        self_lock.read().unwrap().graph[node_index].execute()?;

                        // Set execution_status for `node_index` to `ExecutionStatus::Executed`.
                        self_lock.write().unwrap().graph[node_index].execution_status = ExecutionStatus::Executed;
                        println!("{:?}: Set execution status to executed.", node_index);

                        // Get indeces of nodes that are now executable (due to all their parent nodes having been executed).
                        let self_data = self_lock.read().unwrap();
                        let new_executable_nodes: Vec<(NodeIndex, ExecutionStatus)> = self_data
                            .graph
                            .neighbors_directed(node_index, Direction::Outgoing)
                            .filter_map(|next_index| {
                                // Nodes that need to be executed prior to executing `next_index` (parent nodes).
                                for parent_index in self_data.graph.neighbors_directed(next_index, Direction::Incoming).collect::<Vec<NodeIndex>>() {
                                    // If one parent node has not been executed, break loop because child is not executable.
                                    if self_data.graph[parent_index].execution_status != ExecutionStatus::Executed {
                                        return None;
                                    }
                                }
                                return Some((next_index, ExecutionStatus::Executable));
                            })
                            .collect();
                        drop(self_data);

                        // Notify all threads if graph was executed.
                        if self_lock.read().unwrap().is_graph_executed() == true {
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
}
