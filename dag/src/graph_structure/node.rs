use anyhow::{anyhow, Error, Result};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug)]
pub struct Node {
    // pub computation: fn(&str) -> anyhow::Result<&str>,
    pub args: String,
    pub executed: bool,
}

impl Node {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for Node {
    fn default() -> Self {
        Node {
            // computation: |args| return Ok(args),
            args: String::from(""),
            executed: false,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Struct Node, Node.args: {}, Node.executed: {}",
            // "function {:?} with args '{}' {} executed",
            // self.computation,
            self.args,
            self.executed
        )
    }
}

impl FromStr for Node {
    type Err = Error;
    /// Parses `Node` from a string like: "Struct Node, Node.args: "", Node.executed: false"
    fn from_str(node_string: &str) -> Result<Self> {
        let mut node = Node {
            args: String::from(""),
            executed: false,
        };

        for part in node_string.trim().split(',') {
            match part {
                // Parsing `Node`'s `args`
                part if part.starts_with("Node.args: ") => {
                    node.args = String::from(
                        part.strip_prefix("Node.args: ")
                            .ok_or_else(|| anyhow!("Node::from_str parsing error: no 'args: ' prefix despite successful check."))?,
                    )
                }
                // Parsing `Node`'s `executed` status
                part if part.starts_with(" Node.executed: ") => {
                    node.executed = if part
                        .strip_prefix(" Node.executed: ")
                        .ok_or_else(|| anyhow!("Node::from_str parsing error: no ' executed: ' prefix despite successful check."))?
                        == "true"
                    {
                        true
                    } else {
                        false
                    }
                }
                _ => (),
            }
        }

        Ok(node)
    }
}
