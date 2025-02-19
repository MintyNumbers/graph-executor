use super::{edge::Edge, execution_status::ExecutionStatus, node::Node};
use anyhow::{anyhow, Error, Result};
use petgraph::{acyclic::Acyclic, dot, graph::NodeIndex, prelude::StableDiGraph, Direction};
use std::{collections::VecDeque, fmt, fs::write, str::FromStr};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DirectedAcyclicGraph {
    // graph: Arc<Mutex<StableDiGraph<Node, i32>>>,
    graph: StableDiGraph<Node, i32>,
}

impl fmt::Display for DirectedAcyclicGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Struct DirectedAcyclicGraph,\n{}",
            dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])
        )
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
        let mut nodes: Vec<Node> = vec![];
        let mut edges: Vec<Edge> = vec![];

        if dag_string.trim().starts_with("digraph") {
            // let (nodes, edges): (Vec<Node>, Vec<Edge>);
            for line in dag_string.trim().split("\n") {
                match line {
                    l if line.trim().as_bytes()[0].is_ascii_digit() && line.trim()[1..].starts_with(" [") => {
                        let a: Vec<&str> = l.split('\"').collect();
                        nodes.push(Node::from_str(
                            *a.get(1).ok_or(anyhow!("DirectedAcyclicGraph::from_str parsing error: No node label."))?,
                        )?);
                    }
                    l if line.trim().as_bytes()[0].is_ascii_digit() && line.trim().as_bytes()[5].is_ascii_digit() => {
                        edges.push(Edge::from_str(l.trim())?);
                    }
                    // _ if line.trim().starts_with(&['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']) => println!("starts_with"),
                    _ => {}
                }
            }
        }

        DirectedAcyclicGraph::new(nodes, edges)
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
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Result<Self> {
        let mut graph = StableDiGraph::<Node, i32>::new();

        // Populate graph with nodes.
        let mut node_indeces: Vec<NodeIndex> = vec![];
        nodes.into_iter().for_each(|n| {
            node_indeces.push(graph.add_node(n));
        });

        // Populate graph with all edges between nodes.
        edges.into_iter().for_each(|edge| {
            if edge.nodes.0 < node_indeces.len() && edge.nodes.1 < node_indeces.len() {
                graph.add_edge(node_indeces[edge.nodes.0], node_indeces[edge.nodes.1], 1 /* edge.weight */);

                // Set `ExecutionStatus` of `edge.nodes.1` to `NonExecutable`.
                graph[node_indeces[edge.nodes.1]].execution_status = ExecutionStatus::NonExecutable;
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

    /// Execute all `Node`s.
    pub fn execute_nodes(&mut self) -> Result<()> {
        let mut executable_nodes = self.get_executable_node_indeces();

        // let mut i = 0;
        while executable_nodes.len() > 0 {
            // println!("\n\n\nloop {}: {}", i, self);
            // i += 1;

            let node_index = executable_nodes
                .pop_front()
                .ok_or(anyhow!("No executable nodes found despite successful check."))?;

            // Execute first node in queue.
            self.graph[node_index].execute()?;

            // Update `execution_status` of next `Node`s to executable if all their parent nodes have been executed.
            let next_indices: Vec<NodeIndex> = self.graph.neighbors_directed(node_index, Direction::Outgoing).collect(); // Nodes that may be executable after executing `node_index`.
            next_indices.into_iter().for_each(|next_index| {
                // Nodes that need to be executed prior to executing `next_index`.
                let parent_indeces: Vec<NodeIndex> = self.graph.neighbors_directed(next_index, Direction::Incoming).collect();
                for parent_index in parent_indeces {
                    // If one parent node has not been executed, break.
                    if self.graph[parent_index].execution_status != ExecutionStatus::Executed {
                        self.graph[next_index].execution_status = ExecutionStatus::NonExecutable;
                        break;
                    }
                    self.graph[next_index].execution_status = ExecutionStatus::Executable;
                }

                // Add `next_index` to `executable_nodes` if all parent nodes have been executed.
                if self.graph[next_index].execution_status == ExecutionStatus::Executable {
                    executable_nodes.push_back(next_index);
                }
            });
        }

        // TODO: Implement parallel node execution.

        // Get executable (parent) nodes.
        // let mut executable_nodes = self.get_executable_node_indeces();

        // Get number of threads.
        // let num_threads = if num_cpus::get() > executable_nodes.len() {
        //     executable_nodes.len() // If more cores than executable nodes, spawn a thread for each executable node.
        // } else {
        //     num_cpus::get() // If more executable nodes than cores, spawn a thread for each core.
        // };

        // Spawn threads.
        // let mut threads = Vec::with_capacity(num_threads);
        // for _ in 0..num_threads {
        //     let _i = executable_nodes.pop_front().ok_or(anyhow!("No executable nodes found."))?;
        //     threads.push(thread::spawn(move || -> Result<()> {
        //        println!("{:?}", i);
        //        &self.graph[_i].execute()?;
        //
        //        Update children's status to executable if all their parent nodes have been executed.
        //        &self.graph.neighbors_directed(_i, Direction::Incoming).filter_map(|_| None::<()>);
        //
        //         std::thread::sleep(std::time::Duration::from_secs(1));
        //         Ok(())
        //     }));
        // }

        // Wait for threads to exit.
        // for t in threads.drain(..) {
        //     let _ = t.join().map_err(|e| anyhow!("Unable to join thread: {:?}", e))?;
        // }

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
}
