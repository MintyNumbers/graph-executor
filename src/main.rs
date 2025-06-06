#![allow(dead_code)]

//! Proof-of concept implementation of a graph executor component that is executed in a topological order.
//! The graph is represented as a directed acyclic graph (DAG) where each node is executed once and the edges
//! represent the order of execution. The goal of this component is the efficient splitting of the computations
//! associated with each node onto multiple CPU cores using multiple threads and processes with the help of
//! shared memory and cross-process synchronisation.

mod graph_structure;
mod shared_memory;
mod shared_memory_graph_execution;

use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
use std::sync::atomic::AtomicU8;

/// Main function.
fn main() -> anyhow::Result<()> {
    let filename_prefix = "mystorage".to_string();
    let mut dag = DirectedAcyclicGraph::new(
        vec![
            (0, Node::new(String::from("-- Node 0 was just executed --"))),
            (1, Node::new(String::from("-- Node 1 was just executed --"))),
            (2, Node::new(String::from("-- Node 2 was just executed --"))),
            (3, Node::new(String::from("-- Node 3 was just executed --"))),
            (4, Node::new(String::from("-- Node 4 was just executed --"))),
            (5, Node::new(String::from("-- Node 5 was just executed --"))),
            (6, Node::new(String::from("-- Node 6 was just executed --"))),
        ],
        vec![
            Edge::new((0, 1)),
            Edge::new((1, 3)),
            Edge::new((4, 3)),
            Edge::new((2, 4)),
            Edge::new((6, 3)),
            Edge::new((5, 4)),
            Edge::new((5, 6)),
        ],
    )?;

    // Execute defined graph
    dag.execute_graph::<Storage<AtomicU8>>(filename_prefix)?;

    Ok(())
}
