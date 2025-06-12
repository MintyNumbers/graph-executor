pub mod edge;
pub mod execution_status;
pub mod graph;
pub mod node;

#[cfg(test)]
mod tests {
    use super::{
        edge::Edge, execution_status::ExecutionStatus, graph::DirectedAcyclicGraph, node::Node,
    };
    use petgraph::graph::NodeIndex;
    use std::{
        collections::{BTreeMap, VecDeque},
        fs::read_to_string,
        str::FromStr,
    };

    // `Edge` tests

    #[test]
    fn edge_compare_equality_from_str_direct_new() {
        let edge_from_str = Edge::from_str("0 -> 1 [ ]").unwrap();
        let edge_direct = Edge {
            parent: String::from("0"),
            child: String::from("1"),
        };
        let edge_new = Edge::new(String::from("0"), String::from("1"));

        assert_eq!(
            edge_from_str, edge_direct,
            "`Edge::from_string()` and `Edge {{}}` initializations are not equal."
        );
        assert_eq!(
            edge_from_str, edge_new,
            "`Edge::from_string()` and `Edge::new()` initializations are not equal."
        );
        assert_eq!(
            edge_direct, edge_new,
            "`Edge {{}}` and `Edge::new()` initializations are not equal."
        );
    }

    // `Node` tests

    #[test]
    fn node_compare_equality_from_str_new_default() {
        let node_from_str =
            Node::from_str("Struct Node, Node.args: , Node.executed: Executable").unwrap();
        let node_new = Node::new(String::from(""));
        let node_default = Node::default();

        assert_eq!(
            node_from_str, node_new,
            "`Node::from_string()` and `Node::new()` initializations are not equal."
        );
        assert_eq!(
            node_from_str, node_default,
            "`Node::from_string()` and `Node::default()` initializations are not equal."
        );
        assert_eq!(
            node_new, node_default,
            "`Node::new()` and `Node::default()` initializations are not equal."
        );
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
            String::from("Trying to execute node which has already been executed."),
            "Wrong/no error when trying to execute node which has `ExecutionStatus::Executed`."
        );
        assert_eq!(
            result_executing.unwrap(),
            (),
            "Unsuccessful when trying to execute node which has `ExecutionStatus::Executing`."
        );
        assert_eq!(
            result_executable.unwrap_err().to_string(),
            String::from("Trying to execute node which is not yet set for execution."),
            "Wrong/no error when trying to execute node which has `ExecutionStatus::Executable`."
        );
        assert_eq!(
            result_non_executable.unwrap_err().to_string(),
            String::from("Trying to execute node which is not executable."),
            "Wrong/no error when trying to execute node which has `ExecutionStatus::NonExecutable`."
        );
    }

    // `ExecutionStatus` tests

    #[test]
    fn execution_status_compare_equality_from_str_direct() {
        let execution_status_from_str = ExecutionStatus::from_str("Executed").unwrap();
        let execution_status_direct = ExecutionStatus::Executed;

        assert_eq!(
            execution_status_from_str, execution_status_direct,
            "`ExecutionStatus::from_string()` and `ExecutionStatus::Executed` initializations are not equal."
        );
    }

    // `DirectedAcyclicGraph` tests

    #[test]
    fn dag_compare_equality_new_from_str_from_bytes() {
        let graph_new = DirectedAcyclicGraph::new(
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

        let graph_from_str = DirectedAcyclicGraph::from_str(&format!("{}", graph_new)).unwrap();
        let graph_from_bytes =
            rmp_serde::from_slice::<DirectedAcyclicGraph>(&rmp_serde::to_vec(&graph_new).unwrap())
                .unwrap();

        assert_eq!(
            graph_new, graph_from_str,
            "`DAG::new()` and `DAG::from_str()` initializations are not equal."
        );
        assert_eq!(
            graph_new, graph_from_bytes,
            "`DAG::new()` and `DAG::from_bytes()` initializations are not equal."
        );
        assert_eq!(
            graph_from_str, graph_from_bytes,
            "`DAG::from_str()` and `DAG::from_bytes()` initializations are not equal."
        );
    }

    #[test]
    fn dag_parse_from_string() {
        let dag_from_file = DirectedAcyclicGraph::from_str(
            &read_to_string("./resources/example-printed-dot-digraph.dot").unwrap(),
        )
        .unwrap();
        let dag_initialized = DirectedAcyclicGraph::new(
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
                (
                    String::from("4"),
                    Node::new(String::from("Node 4 was just executed")),
                ),
                (
                    String::from("5"),
                    Node::new(String::from("Node 5 was just executed")),
                ),
                (
                    String::from("6"),
                    Node::new(String::from("Node 6 was just executed")),
                ),
            ]),
            vec![
                Edge::new(String::from("0"), String::from("1")),
                Edge::new(String::from("1"), String::from("3")),
                Edge::new(String::from("4"), String::from("3")),
                Edge::new(String::from("2"), String::from("4")),
                Edge::new(String::from("6"), String::from("3")),
                Edge::new(String::from("5"), String::from("4")),
                Edge::new(String::from("5"), String::from("6")),
            ],
        )
        .unwrap();
        assert_eq!(
            dag_from_file, dag_initialized,
            "DAG parsed from file and initialized manually not equal"
        );

        let dag_from_file_2 = DirectedAcyclicGraph::from_str(
            &read_to_string("./resources/example-typical-dot-digraph.dot").unwrap(),
        )
        .unwrap();
        let dag_initialized_2 = DirectedAcyclicGraph::new(
            BTreeMap::from([
                (String::from("a"), Node::new("a".to_string())),
                (String::from("b"), Node::new("b".to_string())),
                (String::from("c"), Node::new("c".to_string())),
                (String::from("d"), Node::new("d".to_string())),
            ]),
            vec![
                Edge::new(String::from("a"), String::from("b")),
                Edge::new(String::from("b"), String::from("c")),
                Edge::new(String::from("b"), String::from("d")),
            ],
        )
        .unwrap();
        assert_eq!(
            dag_from_file_2, dag_initialized_2,
            "DAG parsed from file and initialized manually not equal"
        );
    }

    #[test]
    fn dag_method_get_executable_node_indeces() {
        let graph = DirectedAcyclicGraph::new(
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

        let executable_nodes_1 = graph.get_executable_node_indices();
        let executable_nodes_2 = VecDeque::from(vec![NodeIndex::new(0), NodeIndex::new(2)]);

        assert_eq!(
            executable_nodes_1, executable_nodes_2,
            "`DAG.get_executable_node_indeces()` method does not return correct node indeces."
        );
    }

    #[test]
    fn dag_fail_directed_cyclic_graph() {
        let err = DirectedAcyclicGraph::new(
            BTreeMap::from([
                (
                    String::from("0"),
                    Node::new(String::from("Node 0 was just executed")),
                ),
                (
                    String::from("1"),
                    Node::new(String::from("Node 1 was just executed")),
                ),
            ]),
            vec![
                Edge::new(String::from("0"), String::from("1")),
                Edge::new(String::from("1"), String::from("0")),
            ],
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            format!("Cyclic graph supplied on NodeIndex(1)"),
            "Cyclic graph is successfully created (it shouldn't be)."
        );
    }

    #[test]
    fn dag_get_parent_child_node_indeces() {
        let graph = DirectedAcyclicGraph::new(
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

        let parents = graph
            .get_parent_node_indices(NodeIndex::new(3))
            .collect::<Vec<NodeIndex>>();
        assert_eq!(
            parents,
            Vec::from([NodeIndex::new(1), NodeIndex::new(2)]),
            "Wrong parents of Node 3."
        );

        let parents = graph
            .get_parent_node_indices(NodeIndex::new(2))
            .collect::<Vec<NodeIndex>>();
        assert_eq!(parents, Vec::new(), "Wrong parents of Node 2.");

        let children = graph
            .get_child_node_indices(NodeIndex::new(2))
            .collect::<Vec<NodeIndex>>();
        assert_eq!(
            children,
            Vec::from([NodeIndex::new(3)]),
            "Wrong children of Node 2."
        );

        let children = graph
            .get_child_node_indices(NodeIndex::new(1))
            .collect::<Vec<NodeIndex>>();
        assert_eq!(
            children,
            Vec::from([NodeIndex::new(3)]),
            "Wrong children of Node 1."
        );
    }
}
