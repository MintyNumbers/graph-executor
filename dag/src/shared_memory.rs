pub mod as_from_bytes;
pub mod c_style_rw_lock;
pub mod iox2_shm_mapping;
pub mod rwlock;
pub mod semaphore;
pub mod shm_graph;
pub mod shm_mapping;

#[cfg(test)]
mod tests {
    use super::{iox2_shm_mapping::Iox2ShmMapping, rwlock};
    use crate::graph_structure::{edge::Edge, graph::DirectedAcyclicGraph, node::Node};
    use anyhow::{anyhow, Result};
    use serde::Serialize;

    // `DirectedAcyclicGraph` shared memory tests

    #[test]
    fn dag_serialize_deserialize() -> Result<()> {
        let graph_new = DirectedAcyclicGraph::new(
            vec![(0, Node::new(String::from("Node 0 executed"))), (1, Node::new(String::from("Node 1 executed")))],
            vec![Edge::new((0, 1))],
        )?;

        let bytes = rmp_serde::to_vec(&graph_new)?;
        let graph_from_bytes = rmp_serde::from_slice::<DirectedAcyclicGraph>(&bytes)?;

        if graph_new != graph_from_bytes {
            return Err(anyhow!(
                "Original DAG and its serialized and then deserialized version are not equal:\n{} != {}",
                graph_new,
                graph_from_bytes
            ));
        }

        Ok(())
    }

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
