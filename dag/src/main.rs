#![allow(dead_code, unused_imports, unused_mut, unused_variables)]

//! Proof-of concept implementation of a graph executor component that is executed in a topological order.
//! The graph is represented as a directed acyclic graph (DAG) where each node is executed once and the edges
//! represent the order of execution. The goal of this component is the efficient splitting of the computations
//! associated with each node onto multiple CPU cores using multiple threads and processes with the help of
//! shared memory and cross-process synchronisation.

mod graph_structure;
mod shared_memory;

use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
use shared_memory::iox2_shm_mapping::Iox2ShmMapping;
use std::sync::atomic::AtomicU8;

/// Main function.
fn main() -> anyhow::Result<()> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <process_number: 1 | 2>", args[0]);
        std::process::exit(1);
    }
    let process_number: u32 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Invalid process number: {}", args[1]);
        std::process::exit(1);
    });

    // Different
    let filename_prefix = "mystorage".to_string();
    match process_number {
        // Process 1
        1 => {
            // shm_process_one::<Memory<PoolAllocator>>()?;
            let mut iox2mapping: Iox2ShmMapping<Storage<AtomicU8>, DirectedAcyclicGraph> = Iox2ShmMapping::new(
                filename_prefix,
                DirectedAcyclicGraph::new(
                    vec![
                        (0, Node::new(String::from("-- Node 0 was just executed --"))),
                        (1, Node::new(String::from("-- Node 1 was just executed --"))),
                        (2, Node::new(String::from("-- Node 2 was just executed --"))),
                        (3, Node::new(String::from("-- Node 3 was just executed --"))),
                    ],
                    vec![Edge::new((0, 1)), /* Edge::new((1, 2)), */ Edge::new((2, 3)), Edge::new((1, 3))],
                )?,
            )?;

            println!("Unlocked...");
            std::thread::sleep(std::time::Duration::from_secs(5));

            iox2mapping.data.graph.add_node(Node::new("rboegbrgoergoierbger".to_string()));
            iox2mapping.write_self_to_shm()?;
            println!("Changed...");
            std::thread::sleep(std::time::Duration::from_secs(5));

            println!("Process 1 executed");
        }
        // Process 2
        2 => {
            // shm_process_two::<Memory<PoolAllocator>>()?;
            Iox2ShmMapping::<Storage<AtomicU8>, DirectedAcyclicGraph>::open_existing(filename_prefix)?;
            println!("Process 2 executed");
        }
        _ => {
            eprintln!("Invalid process number: {}", process_number);
            std::process::exit(1);
        }
    }

    Ok(())
}
