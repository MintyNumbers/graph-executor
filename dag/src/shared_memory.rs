pub mod rwlock;
pub mod semaphore;
pub mod shm_mapping;

#[cfg(test)]
mod tests {
    use super::{rwlock, semaphore::Semaphore, shm_mapping::ShmMapping};
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
}
