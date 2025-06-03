use super::{edge::Edge, execution_status::ExecutionStatus, node::Node};
use anyhow::{anyhow, Error, Ok, Result};
use petgraph::{acyclic::Acyclic, dot, graph::NodeIndex, prelude::StableDiGraph};
use std::{collections::HashMap, collections::VecDeque, fmt, fs::write, ops::Index, ops::IndexMut, str::FromStr};

/// This struct is a wrapper for `petgraph`'s `StableDiGraph` implementation.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DirectedAcyclicGraph {
    /// `petgraph`'s `StableDiGraph`.
    pub(crate) graph: StableDiGraph<Node, i32>,
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
    pub(crate) fn get_executable_node_indeces(&self) -> VecDeque<NodeIndex> {
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
    pub(crate) fn get_executable_node_index(&self) -> Option<NodeIndex> {
        self.graph
            .node_indices()
            .find(|i| self.graph[*i].execution_status == ExecutionStatus::Executable)
    }

    /// Checks whether all nodes have been executed.
    pub fn is_graph_executed(&self) -> bool {
        self.graph
            .node_weights()
            .filter_map(|n| if n.execution_status == ExecutionStatus::Executed { None } else { Some(n) })
            .collect::<Vec<&Node>>()
            .is_empty()
    }
}
