pub mod execute_graph;
pub mod shm_graph;

#[cfg(test)]
mod tests {
    use crate::graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
    use iceoryx2_cal::dynamic_storage::posix_shared_memory::Storage;
    use std::sync::atomic::AtomicU8;

    #[test]
    fn dag_method_execute_nodes_one_process() {
        let mut dag = DirectedAcyclicGraph::new(
            vec![
                (0, Node::new(String::from("Node 0 was just executed"))),
                (1, Node::new(String::from("Node 1 was just executed"))),
                (2, Node::new(String::from("Node 2 was just executed"))),
                (3, Node::new(String::from("Node 3 was just executed"))),
            ],
            vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
        )
        .unwrap();
        dag.execute_graph::<Storage<AtomicU8>>(String::from("test_shared_memory")).unwrap();

        assert_eq!(dag.is_graph_executed(), true, "`shm.execute_graph()` method does not execute all `Node`s.");
    }
}
