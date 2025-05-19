pub mod execute_graph;

#[cfg(test)]
mod tests {
    // #[test]
    // fn dag_method_execute_nodes() {
    //     let mut shm = ShmMapping::new(
    //         String::from("test_flink"),
    //         DirectedAcyclicGraph::new(
    //             vec![
    //                 (0, Node::new(String::from("Node 0 was just executed"))),
    //                 (1, Node::new(String::from("Node 1 was just executed"))),
    //                 (2, Node::new(String::from("Node 2 was just executed"))),
    //                 (3, Node::new(String::from("Node 3 was just executed"))),
    //             ],
    //             vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
    //         )
    //         .unwrap(),
    //         false,
    //     )
    //     .unwrap();

    //     shm.execute_graph().unwrap();

    //     assert_eq!(
    //         shm.wrapped.is_graph_executed(),
    //         true,
    //         "`shm.execute_graph()` method does not execute all `Node`s."
    //     );
    // }
}
