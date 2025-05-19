#![allow(dead_code, unused_imports, unused_mut, unused_variables)]

//! Proof-of concept implementation of a graph executor component that is executed in a topological order.
//! The graph is represented as a directed acyclic graph (DAG) where each node is executed once and the edges
//! represent the order of execution. The goal of this component is the efficient splitting of the computations
//! associated with each node onto multiple CPU cores using multiple threads and processes with the help of
//! shared memory and cross-process synchronisation.

mod graph_structure;
mod shared_memory;
mod shm_graph_execution;

use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
use shared_memory::shm_mapping::ShmMapping;
use shm_graph_execution::execute_graph;
use std::sync::atomic::AtomicU8;

/// Main function.
fn main() -> anyhow::Result<()> {
    // Collect command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} < process_number: 1 | 2 | 3 >", args[0]);
        std::process::exit(1);
    }
    let process_number: u32 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Invalid process number: {}", args[1]);
        std::process::exit(1);
    });

    // Different
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
    match process_number {
        // Process 1
        1 => {
            let mut shm_mapping = ShmMapping::<Storage<AtomicU8>>::new(&filename_prefix, &dag)?;
            // println!("Initial write complete: {} {}", shm_mapping.data_storages.len(), dag);
            std::thread::sleep(std::time::Duration::from_secs(5));

            // dag.graph.add_node(Node::new("Dynamically added new node".to_string()));
            // shm_mapping.write(&dag)?;
            // println!("Changed...");
            // std::thread::sleep(std::time::Duration::from_secs(5));

            let data = shm_mapping.read::<DirectedAcyclicGraph>()?;
            // println!("data: {}, {}", shm_mapping.data_storages.len(), data);

            println!("Process 1 executed");
        }
        // Process 2
        2 => {
            let (mut shm_mapping_2, mut data) = ShmMapping::<Storage<AtomicU8>>::open::<DirectedAcyclicGraph>(&filename_prefix)?;
            // println!("Data from shm: {} {}", shm_mapping_2.data_storages.len(), data);

            // for i in 0..50 {
            //     data = shm_mapping_2.read()?;
            //     println!("{}", data);
            //     std::thread::sleep(std::time::Duration::from_secs(1));
            // }

            data.graph.add_node(Node::new("ahahhahahha".to_string()));
            shm_mapping_2.write(&data)?;
            // println!("New data: {}, {}", shm_mapping_2.data_storages.len(), data);
            std::thread::sleep(std::time::Duration::from_secs(6));

            println!("Process 2 executed");
        }
        3 => {
            execute_graph::execute_graph(filename_prefix, dag)?;
        }
        _ => {
            eprintln!("Invalid process number: {}", process_number);
            std::process::exit(1);
        }
    }

    Ok(())
}
