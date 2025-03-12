pub mod as_from_bytes;
pub mod shm_graph;
pub mod shm_mapping;

#[cfg(test)]
mod tests {
    use super::as_from_bytes::AsFromBytes;
    use crate::{
        graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node},
        shared_memory::shm_mapping::ShmMapping,
    };

    // `DirectedAcyclicGraph` shared memory tests

    #[test]
    fn dag_from_bytes() {
        let graph_new = DirectedAcyclicGraph::new(
            vec![(0, Node::new(String::from("Node 0 executed"))), (1, Node::new(String::from("Node 1 executed")))],
            vec![Edge::new((0, 1))],
        )
        .unwrap();

        let graph_bytes = graph_new.as_bytes();
        let graph_from_bytes = DirectedAcyclicGraph::from_bytes(graph_bytes);

        assert_eq!(
            graph_bytes.len(),
            size_of::<DirectedAcyclicGraph>(),
            "Byte slice length of `DAG` is not equal to `size_of::<DAG>`."
        );
        assert_eq!(
            graph_new, graph_from_bytes,
            "`DAG::new()` and `DAG::from_bytes()` initializations are not equal."
        );
    }

    #[test]
    fn dag_method_execute_nodes() {
        let mut shm = ShmMapping::new(
            String::from("test_flink"),
            DirectedAcyclicGraph::new(
                vec![
                    (0, Node::new(String::from("Node 0 was just executed"))),
                    (1, Node::new(String::from("Node 1 was just executed"))),
                    (2, Node::new(String::from("Node 2 was just executed"))),
                    (3, Node::new(String::from("Node 3 was just executed"))),
                ],
                vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
            )
            .unwrap(),
            false,
        )
        .unwrap();

        shm.execute_graph().unwrap();

        assert_eq!(
            shm.wrapped.is_graph_executed(),
            true,
            "`shm.execute_graph()` method does not execute all `Node`s."
        );
    }
}
