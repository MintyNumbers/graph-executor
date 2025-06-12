pub mod execute_graph;
pub mod shm_graph;

#[cfg(test)]
mod tests {
    use crate::graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
    use std::collections::BTreeMap;

    #[test]
    fn dag_method_execute_nodes_one_process() {
        let mut dag = DirectedAcyclicGraph::new(
            BTreeMap::from([
                (
                    String::from("0"),
                    Node::new(String::from("Node 0 was just executed")),
                ),
                (
                    String::from("1"),
                    Node::new(String::from("Node 1 was just executed")),
                ),
                (
                    String::from("2"),
                    Node::new(String::from("Node 2 was just executed")),
                ),
                (
                    String::from("3"),
                    Node::new(String::from("Node 3 was just executed")),
                ),
            ]),
            vec![
                Edge::new(String::from("0"), String::from("1")),
                Edge::new(String::from("2"), String::from("3")),
                Edge::new(String::from("1"), String::from("3")),
            ],
        )
        .unwrap();
        dag.execute(String::from("test_shared_memory")).unwrap();

        assert_eq!(
            dag.is_graph_executed(),
            true,
            "`shm.execute_graph()` method does not execute all `Node`s."
        );
    }
}
