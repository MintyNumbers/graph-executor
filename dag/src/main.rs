//! Proof-of concept implementation of a graph executor component that is executed in a topological order.
//! The graph is represented as a directed acyclic graph (DAG) where each node is executed once and the edges
//! represent the order of execution. The goal of this component is the efficient splitting of the computations
//! associated with each node onto multiple CPU cores using multiple threads and processes with the help of
//! shared memory and cross-process synchronisation.

mod graph_structure;
mod shared_memory;

use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
use shared_memory::shm_mapping::ShmMapping;
use std::{fs::read_to_string, str::FromStr};

/// Main function.
fn main() -> anyhow::Result<()> {
    // Create new `DirectedAcyclicGraph` with nodes and edges that are moved into the graph.
    let g = DirectedAcyclicGraph::new(
        vec![
            (0, Node::new(String::from("-- Node 0 was just executed --"))),
            (1, Node::new(String::from("-- Node 1 was just executed --"))),
            (2, Node::new(String::from("-- Node 2 was just executed --"))),
            (3, Node::new(String::from("-- Node 3 was just executed --"))),
        ],
        vec![Edge::new((0, 1)), /* Edge::new((1, 2)), */ Edge::new((2, 3)), Edge::new((1, 3))],
    )?;

    // Write the created graph to resources/example.dot.
    g.write_to_path("resources/example.dot")?;

    // Parse the `DirectedAcyclicGraph` from `String`.
    let f = DirectedAcyclicGraph::from_str(read_to_string("resources/example.dot")?.as_str())?;

    // Create shared memory mapping with `DirectedAcyclicGraph`.
    let mut shm_mapping = ShmMapping::new(String::from("shared_mem_mapping"), f, false)?;

    // Execute graph.
    shm_mapping.execute_graph()?;

    Ok(())
}
