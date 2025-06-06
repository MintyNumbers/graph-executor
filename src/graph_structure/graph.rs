use super::{edge::Edge, execution_status::ExecutionStatus, node::Node};
use anyhow::{anyhow, Error, Ok, Result};
use petgraph::{
    acyclic::Acyclic, dot, graph::NodeIndex, prelude::StableDiGraph, stable_graph::Neighbors,
    Direction,
};
use std::{
    collections::BTreeMap, collections::VecDeque, fmt, fs::read_to_string, fs::write, ops::Index,
    ops::IndexMut, str::FromStr,
};

/// This struct is a wrapper for `petgraph`'s `StableDiGraph` implementation.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DirectedAcyclicGraph {
    /// `petgraph`'s `StableDiGraph`.
    graph: StableDiGraph<Node, i32>,
}

impl fmt::Display for DirectedAcyclicGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
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
        // Vectors for future `node`s and `edge`s of the new `DirectedAcyclicGraph`
        let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
        let mut edges: Vec<Edge> = vec![];

        if dag_string.trim().starts_with("digraph") {
            for line in dag_string.trim().split("\n") {
                let line = {
                    if line.ends_with(";") {
                        line.strip_suffix(";")
                            .ok_or(anyhow!("No ; suffix despite successful check."))?
                    } else {
                        line
                    }
                };

                let line_split_space = line
                    .trim()
                    .split(" ")
                    .map(|s| s.trim())
                    .collect::<Vec<&str>>();

                // Parse line as `Node` if it looks like:
                // 0 [ label = "Struct Node, Node.args: -- Node 0 was just executed --, Node.execution_status: Executable" ]
                if line_split_space.len() >= 6 && line_split_space[0].chars().all(|c| c.is_ascii_digit()) // 0
                    && line_split_space[1] == "["                                // [
                    && line_split_space[2] == "label"                            // label
                    && line_split_space[3] == "="                                // =
                    && line_split_space[4] == "\"Struct"                         // "Struct
                    && line_split_space[5] == "Node,"                            // Node,
                    && line_split_space[6] == "Node.args:"
                // Node.args:
                {
                    nodes.insert(
                        line_split_space[0].to_string(),
                        Node::from_str(*line.split('\"').collect::<Vec<&str>>().get(1).ok_or(
                            anyhow!("DirectedAcyclicGraph::from_str parsing error: No node label."),
                        )?)?,
                    );
                }
                // Parse line as `Edge` if it looks like:
                // 0 -> 1 [ ]
                else if line_split_space.len() >= 4 && line_split_space[0].chars().all(|c| c.is_ascii_digit()) // 0
                    && line_split_space[1] == "->"                                    // ->
                    && line_split_space[2].chars().all(|c| c.is_ascii_digit())  // 1
                    && line_split_space[3] == "["                                     // [
                    && line_split_space[4] == "]"
                // ]
                {
                    edges.push(Edge::new((
                        line_split_space[0].to_string(),
                        line_split_space[2].to_string(),
                    )));
                }
                // Parse line as `Edge` and `Node` if it looks like the compact DOT syntax:
                // a -> b -> c;
                else if line_split_space.len() >= 3 && line_split_space[1] == "->" {
                    let line_split_arrow = line
                        .split("->")
                        .into_iter()
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<String>>();
                    for (node_num, node_str_identifier) in line_split_arrow.iter().enumerate() {
                        // Insert every node in chain a -> b -> c if it isn't included yet
                        if !nodes.contains_key(node_str_identifier) {
                            nodes.insert(
                                node_str_identifier.clone(),
                                Node::new(node_str_identifier.clone()),
                            );
                        }
                        // Insert edge
                        if node_num >= 1 {
                            edges.push(Edge::new((
                                line_split_arrow[node_num - 1].to_string(),
                                line_split_arrow[node_num].to_string(),
                            )));
                        }
                    }
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
        if self.graph.node_indices().count() != other.graph.node_indices().count()
            || self.graph.edge_indices().count() != other.graph.edge_indices().count()
        {
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
    pub fn new(nodes: BTreeMap<String, Node>, edges: Vec<Edge>) -> Result<Self> {
        let mut graph = StableDiGraph::<Node, i32>::new();

        // Populate graph with all nodes.
        let node_string_id_to_node_index_map: BTreeMap<String, NodeIndex> = nodes
            .into_iter()
            .map(|(string_id, node)| (string_id, graph.add_node(node)))
            .collect();

        // Populate graph with all edges between nodes.
        edges.into_iter().for_each(|edge| {
            if node_string_id_to_node_index_map.contains_key(&edge.nodes.0)
                && node_string_id_to_node_index_map.contains_key(&edge.nodes.1)
            {
                graph.add_edge(
                    node_string_id_to_node_index_map[&edge.nodes.0],
                    node_string_id_to_node_index_map[&edge.nodes.1],
                    1,
                );

                // Set `ExecutionStatus` of child nodes to `NonExecutable`.
                graph[node_string_id_to_node_index_map[&edge.nodes.1]].execution_status =
                    ExecutionStatus::NonExecutable;
            } else {
                println!(
                    "One or more of nodes of edge is not defined as a node: {:?}",
                    edge
                );
            }
        });

        // Check that `StableDiGraph` is acyclic and return `DirectedAcyclicGraph` if successful.
        Acyclic::try_from_graph(&graph)
            .map_err(|e| anyhow!("Cyclic graph supplied on {:?}", e.node_id()))?;
        Ok(DirectedAcyclicGraph { graph: graph })
    }

    pub fn from_file(digraph_file: &str) -> Result<Self> {
        Ok(DirectedAcyclicGraph::from_str(
            &read_to_string(digraph_file)
                .map_err(|e| anyhow!("Failed reading file {}: {}", digraph_file, e))?,
        )?)
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
        write(
            path,
            &format!(
                "{}",
                dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])
            ),
        )?;
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
            .filter_map(|n| {
                if n.execution_status == ExecutionStatus::Executed {
                    None
                } else {
                    Some(n)
                }
            })
            .collect::<Vec<&Node>>()
            .is_empty()
    }

    pub fn get_parent_node_indeces(&self, index: NodeIndex) -> Neighbors<'_, i32> {
        self.graph.neighbors_directed(index, Direction::Incoming)
    }

    pub fn get_child_node_indeces(&self, index: NodeIndex) -> Neighbors<'_, i32> {
        self.graph.neighbors_directed(index, Direction::Outgoing)
    }
}
