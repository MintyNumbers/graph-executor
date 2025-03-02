pub mod edge;
pub mod execution_status;
pub mod graph;
pub mod node;

#[cfg(test)]
mod tests {
    use petgraph::graph::NodeIndex;

    use super::{edge::Edge, execution_status::ExecutionStatus, graph::DirectedAcyclicGraph, node::Node};
    use std::{collections::VecDeque, str::FromStr};

    // `Edge` tests

    #[test]
    fn edge_compare_equality_from_str_direct_new() {
        let edge_from_str = Edge::from_str("0 -> 1 [ ]").unwrap();
        let edge_direct = Edge { nodes: (0, 1) };
        let edge_new = Edge::new((0, 1));

        assert_eq!(edge_from_str, edge_direct);
        assert_eq!(edge_from_str, edge_new);
        assert_eq!(edge_direct, edge_new);
    }

    // `Node` tests

    #[test]
    fn node_compare_equality_from_str_new_default() {
        let node_from_str = Node::from_str("Struct Node, Node.args: , Node.executed: Executable").unwrap();
        let node_new = Node::new(String::from(""));
        let node_default = Node::default();

        assert_eq!(node_from_str, node_new);
        assert_eq!(node_from_str, node_default);
        assert_eq!(node_new, node_default);
    }

    #[test]
    fn node_method_execute() {
        let mut node_executed = Node::new(String::from(""));
        node_executed.execution_status = ExecutionStatus::Executed;
        let mut node_executing = Node::new(String::from(""));
        node_executing.execution_status = ExecutionStatus::Executing;
        let node_executable = Node::new(String::from(""));
        let mut node_non_executable = Node::new(String::from(""));
        node_non_executable.execution_status = ExecutionStatus::NonExecutable;

        let result_executed = node_executed.execute();
        let result_executing = node_executing.execute();
        let result_executable = node_executable.execute();
        let result_non_executable = node_non_executable.execute();

        assert_eq!(
            result_executed.unwrap_err().to_string(),
            String::from("Trying to execute node which has already been executed.")
        );
        assert_eq!(result_executing.unwrap(), ());
        assert_eq!(
            result_executable.unwrap_err().to_string(),
            String::from("Trying to execute node which is not yet set for execution.")
        );
        assert_eq!(
            result_non_executable.unwrap_err().to_string(),
            String::from("Trying to execute node which is not executable.")
        );
    }

    // `ExecutionStatus` tests

    #[test]
    fn execution_status_compare_equality_from_str_direct() {
        let execution_status_from_str = ExecutionStatus::from_str("Executed").unwrap();
        let execution_status_direct = ExecutionStatus::Executed;

        assert_eq!(execution_status_from_str, execution_status_direct);
    }

    // `DirectedAcyclicGraph` tests

    #[test]
    fn dag_compare_equality_new_from_str_from_bytes() {
        let graph = DirectedAcyclicGraph::new(
            vec![
                (0, Node::new(String::from("Node 0 was just executed"))),
                (1, Node::new(String::from("Node 1 was just executed"))),
                (2, Node::new(String::from("Node 2 was just executed"))),
                (3, Node::new(String::from("Node 3 was just executed"))),
            ],
            vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
        )
        .unwrap();

        let graph_from_str = DirectedAcyclicGraph::from_str(&format!("{}", graph)).unwrap();
        let graph_from_bytes = unsafe { DirectedAcyclicGraph::from_bytes(graph.as_bytes()) };

        assert_eq!(graph, graph_from_str);
        assert_eq!(graph, *graph_from_bytes);
    }

    #[test]
    fn dag_method_get_executable_node_indeces() {
        let graph = DirectedAcyclicGraph::new(
            vec![
                (0, Node::new(String::from("Node 0 was just executed"))),
                (1, Node::new(String::from("Node 1 was just executed"))),
                (2, Node::new(String::from("Node 2 was just executed"))),
                (3, Node::new(String::from("Node 3 was just executed"))),
            ],
            vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
        )
        .unwrap();

        let executable_nodes_1 = graph.get_executable_node_indeces();
        let executable_nodes_2 = VecDeque::from(vec![NodeIndex::new(0), NodeIndex::new(2)]);

        assert_eq!(executable_nodes_1, executable_nodes_2);
    }

    #[test]
    fn dag_method_execute_nodes() {
        let mut graph = DirectedAcyclicGraph::new(
            vec![
                (0, Node::new(String::from("Node 0 was just executed"))),
                (1, Node::new(String::from("Node 1 was just executed"))),
                (2, Node::new(String::from("Node 2 was just executed"))),
                (3, Node::new(String::from("Node 3 was just executed"))),
            ],
            vec![Edge::new((0, 1)), Edge::new((2, 3)), Edge::new((1, 3))],
        )
        .unwrap();

        graph.execute_nodes().unwrap();

        assert_eq!(graph.is_graph_executed(), true);
    }
}
