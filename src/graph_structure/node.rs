use super::execution_status::ExecutionStatus;
use anyhow::{anyhow, Error, Result};
use std::{fmt, str::FromStr, thread, time::Duration};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Execution placeholder prior to implementing arbitrary computation execution.
    args: String,
    /// The execution status indicates, whether a node is executable / is currently executing / has already been executed.
    /// Changes during the [`Node`]'s lifetime in the following order:
    ///
    /// 1. [`ExecutionStatus::NonExecutable`] if the node has at least one parent node which hasn't been executed.
    /// 2. [`ExecutionStatus::Executable`] if no parent node hasn't been executed.
    /// 3. [`ExecutionStatus::Executing`] if some process started executing this node.
    /// 4. [`ExecutionStatus::Executed`] if the process has finished executing.
    pub(crate) execution_status: ExecutionStatus,
}

impl Node {
    /// Creates a new [`Node`].
    pub fn new(args: String) -> Self {
        Node {
            args: args,
            execution_status: ExecutionStatus::Executable,
        }
    }
}

impl Default for Node {
    /// Constructs a default Node instance with empty args.
    fn default() -> Self {
        Node {
            args: String::from(""),
            execution_status: ExecutionStatus::Executable,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Struct Node, Node.args: {}, Node.execution_status: {}",
            self.args, self.execution_status
        )
    }
}

impl FromStr for Node {
    type Err = Error;
    /// Parses [`Node`] from a string like: "Struct Node, Node.args: , Node.execution_status: Executable"
    ///
    /// The following two [`Node`]s are identical:
    /// ```
    /// let node_from_str = Node::from_str("Struct Node, Node.args: , Node.execution_status: Executable").unwrap();
    /// let node_new = Node::new(String::from(""));
    /// ```
    fn from_str(node_string: &str) -> Result<Self> {
        let mut node = Node {
            args: String::from(""),
            execution_status: ExecutionStatus::Executable,
        };

        for part in node_string.trim().split(',') {
            match part {
                // Parsing `Node`'s `args`.
                part if part.starts_with(" Node.args: ") => {
                    node.args = String::from(part.strip_prefix(" Node.args: ").ok_or(anyhow!(
                        "Node::from_str parsing error: no 'args: ' prefix despite successful check."
                    ))?)
                }
                // Parsing `Node`'s `execution_status`.
                part if part.starts_with(" Node.execution_status: ") => {
                    node.execution_status =
                        ExecutionStatus::from_str(part.strip_prefix(" Node.execution_status: ").ok_or(anyhow!(
                            "Node::from_str parsing error: no ' execution_status: ' prefix despite successful check."
                        ))?)?;
                }
                _ => (),
            }
        }

        Ok(node)
    }
}

impl Node {
    /// Executes a [`Node`]'s associated computation (currently: printing `Node.args`).
    pub(crate) fn execute(&self) -> Result<()> {
        match self.execution_status {
            ExecutionStatus::Executed => {
                return Err(anyhow!(
                    "Trying to execute node which has already been executed."
                ))
            }
            ExecutionStatus::Executable => {
                return Err(anyhow!(
                    "Trying to execute node which is not yet set for execution."
                ))
            }
            ExecutionStatus::NonExecutable => {
                return Err(anyhow!("Trying to execute node which is not executable."))
            }
            ExecutionStatus::Executing => {
                thread::sleep(Duration::from_secs(1)); // Sleep if no executable `Node` is available
                println!("{}", self.args); // TODO: implement node execution.
                Ok(())
            }
        }
    }
}
