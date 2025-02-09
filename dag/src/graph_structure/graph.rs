use super::{edge::Edge, node::Node};
use anyhow::{anyhow, Error, Result};
use petgraph::{
    acyclic::Acyclic,
    dot,
    stable_graph::{NodeIndex, StableDiGraph},
};
use std::{fs::write, str::FromStr};

#[derive(Clone, Debug)]
pub struct DirectedAcyclicGraph {
    graph: Acyclic<StableDiGraph<Node, i32>>,
}

impl FromStr for DirectedAcyclicGraph {
    type Err = Error;
    fn from_str(dag_string: &str) -> Result<Self> {
        let mut nodes: Vec<Node> = vec![];
        let mut edges: Vec<Edge> = vec![];

        if dag_string.trim().starts_with("digraph") {
            // let (nodes, edges): (Vec<Node>, Vec<Edge>);
            for (line_number, line) in dag_string.trim().split("\n").enumerate() {
                println!("\n{}: {}", line_number, line);
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
                    _ => println!("neither"),
                }
            }
        }

        println!("{:?}", nodes);
        println!("{:?}", edges);

        Ok(DirectedAcyclicGraph {
            graph: Acyclic::<StableDiGraph<Node, i32>>::new(),
        })
    }
}

impl DirectedAcyclicGraph {
    /// Creates `Acyclic<StableGraph<Node, i32>>` from `nodes` and `node_edges`.
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Result<Self> {
        let mut graph = StableDiGraph::<Node, i32>::new();

        // populate graph with nodes
        let mut node_indeces: Vec<NodeIndex> = vec![];
        nodes.into_iter().for_each(|n| {
            node_indeces.push(graph.add_node(n));
        });

        // populate graph with all edges between nodes
        edges.into_iter().for_each(|edge| {
            if edge.nodes.0 < node_indeces.len() && edge.nodes.1 < node_indeces.len() {
                graph.add_edge(node_indeces[edge.nodes.0], node_indeces[edge.nodes.1], 1 /* edge.weight */);
            }
        });

        // println!("{:?}", dot::Dot::with_config(&graph, &[dot::Config::EdgeNoLabel]));

        // cast `StableDiGraph` as an `Acyclic<StableDiGraph>`
        let graph = Acyclic::try_from_graph(graph).map_err(|e| anyhow!("Cyclic graph supplied on {:?}", e.node_id()))?;
        Ok(DirectedAcyclicGraph { graph: graph })
    }

    /// Write `StableGraph` to `path`.
    pub fn write_to_path(&self, path: &str) -> Result<()> {
        write(path, &format!("{}", dot::Dot::with_config(&self.graph, &[dot::Config::EdgeNoLabel])))?;
        Ok(())
    }
}
