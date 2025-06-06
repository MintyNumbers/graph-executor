#![allow(dead_code)]

//! Proof-of concept implementation of a graph executor component that is executed in a topological order.
//! The graph is represented as a directed acyclic graph (DAG) where each node is executed once and the edges
//! represent the order of execution. The goal of this component is the efficient splitting of the computations
//! associated with each node onto multiple CPU cores using multiple threads and processes with the help of
//! shared memory and cross-process synchronisation.

mod graph_structure;
mod shared_memory;
mod shared_memory_graph_execution;

use anyhow::anyhow;
use graph_structure::graph::DirectedAcyclicGraph;
use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
use std::{fs::read_to_string, str::FromStr, sync::atomic::AtomicU8};

/// Main function.
fn main() -> anyhow::Result<()> {
    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        Err(anyhow!("Usage: {} <digraph_file> <filename_suffix>\nExample: {} ./resources/example-printed-dot-digraph.dot test", args[0], args[0]))?;
    }
    let digraph_file: String = args[1]
        .parse()
        .map_err(|e| anyhow!("Error parsing digraph file {}: {}", args[1], e))?;
    let filename_suffix: String = args[2]
        .parse()
        .map_err(|e| anyhow!("Invalid filename suffix {}: {}", args[2], e))?;

    // Read digraph from file and execute it
    let mut dag = DirectedAcyclicGraph::from_str(
        &read_to_string(&digraph_file)
            .map_err(|e| anyhow!("Failed reading file {}: {}", digraph_file, e))?,
    )?;
    dag.execute_graph::<Storage<AtomicU8>>(filename_suffix)?;

    Ok(())
}
