mod graph_structure;
use graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
use std::{fs::read_to_string, str::FromStr};

fn main() -> anyhow::Result<()> {
    // Create new Directed Acyclic Graph
    let g = DirectedAcyclicGraph::new(
        vec![Node::new(), Node::new(), Node::new(), Node::new()],
        // vec![Edge::new((0, 1), 1), Edge::new((1, 2), 7), Edge::new((2, 3), 1), Edge::new((1, 3), 3)],
        vec![Edge::new((0, 1)), Edge::new((1, 2)), Edge::new((2, 3)), Edge::new((1, 3))],
    )?;

    // Write the created graph to resources/example.dot
    g.write_to_path("resources/example.dot")?;

    // Parse the Directed Acyclic Graph from String
    let _ = DirectedAcyclicGraph::from_str(read_to_string("resources/example.dot")?.as_str())?;

    Ok(())
}
