mod graph_structure;
mod shared_memory;

use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
// use graph_structure::graph::DirectedAcyclicGraph;
use shared_memory::shm_mapping::ShmMapping;
use std::{fs::read_to_string, str::FromStr};

fn main() -> anyhow::Result<()> {
    // Create new `DirectedAcyclicGraph`.
    let g = DirectedAcyclicGraph::new(
        vec![
            Node::new(String::from("Node 0 was just executed")),
            Node::new(String::from("Node 1 was just executed")),
            Node::new(String::from("Node 2 was just executed")),
            Node::new(String::from("Node 3 was just executed")),
        ],
        vec![Edge::new((0, 1)), /* Edge::new((1, 2)), */ Edge::new((2, 3)), Edge::new((1, 3))],
    )?;

    // Write the created graph to resources/example.dot.
    g.write_to_path("resources/example.dot")?;

    // Parse the `DirectedAcyclicGraph` from `String`.
    let f = DirectedAcyclicGraph::from_str(read_to_string("resources/example.dot")?.as_str())?;

    // Create shared memory mapping with `DirectedAcyclicGraph`.
    let mut shm_mapping = ShmMapping::new(String::from("shared_mem_mapping"), f)?;

    // Execute graph.
    shm_mapping.execute_graph()?;

    Ok(())
}
