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

    // `Semaphore` and `rwlock` tests

    #[test]
    fn rwlock() -> Result<()> {
        // Create RwLock
        let filename_prefix = "cargo_test";
        let write_lock = Semaphore::create(&format!("/{}_write_lock_write", filename_prefix), 1).map_err(|e| anyhow!("Failed to create write_lock: {}", e))?;
        let read_count = Semaphore::create(&format!("/{}_read_count_write", filename_prefix), 0).map_err(|e| anyhow!("Failed to create read_count: {}", e))?;
        assert_eq!(
            write_lock
                .get_value()
                .map_err(|e| anyhow!("Failed getting write_lock semaphore value: {}", e))?,
            1,
            "write_lock semaphore not equal to 1 after initialization."
        );
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            0,
            "read_count semaphore not equal to 0 after initialization."
        );

        rwlock::read_lock(&write_lock, &read_count)?;
        assert_eq!(
            write_lock
                .get_value()
                .map_err(|e| anyhow!("Failed getting write_lock semaphore value: {}", e))?,
            1,
            "write_lock semaphore changed (not equal to 1) after registering new reader."
        );
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            1,
            "read_count semaphore not equal to 1 after registering new reader."
        );

        rwlock::read_lock(&write_lock, &read_count)?;
        assert_eq!(
            write_lock
                .get_value()
                .map_err(|e| anyhow!("Failed getting write_lock semaphore value: {}", e))?,
            1,
            "write_lock semaphore changed (not equal to 1) after registering new reader."
        );
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            2,
            "read_count semaphore not equal to 2 after registering new reader."
        );

        rwlock::read_unlock(&read_count)?;
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            1,
            "read_count semaphore not equal to 1 after unregistering active reader."
        );

        rwlock::read_unlock(&read_count)?;
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            0,
            "read_count semaphore not equal to 0 after unregistering active reader."
        );

        rwlock::write_lock(&write_lock, &read_count)?;
        assert_eq!(
            write_lock
                .get_value()
                .map_err(|e| anyhow!("Failed getting write_lock semaphore value: {}", e))?,
            0,
            "write_lock semaphore not equal to 0 after registering writer."
        );
        assert_eq!(
            read_count
                .get_value()
                .map_err(|e| anyhow!("Failed getting read_count semaphore value: {}", e))?,
            0,
            "read_count semaphore not equal to 0 after registering writer."
        );

        rwlock::write_unlock(&write_lock)?;
        assert_eq!(
            write_lock
                .get_value()
                .map_err(|e| anyhow!("Failed getting write_lock semaphore value: {}", e))?,
            1,
            "write_lock semaphore not equal to 1 after unregistering writer."
        );

        Ok(())
    }
}
