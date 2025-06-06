use anyhow::{anyhow, Error, Result};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Directed edge (connection) between two nodes.
    /// First index indicates the parent and the second the child node.
    pub(crate) parent: String,
    pub(crate) child: String,
    // pub weight: i32,
}

impl Edge {
    /// Creates new `Edge` from two node indeces returned by `StableDiGraph` when adding `Node`s.
    pub fn new(parent: String, child: String /* , weight: i32 */) -> Self {
        Edge {
            parent,
            child,
            // weight: weight,
        }
    }
}

impl FromStr for Edge {
    type Err = Error;
    /// Parses `Edge` from a string like: "0 -> 1 [ ]"
    ///
    /// The following two `Edge`s are identical:
    /// ```
    /// let edge_from_str = Edge::from_str("0 -> 1 [ ]").unwrap();
    /// let edge_new = Edge::new((0, 1));
    /// ```
    fn from_str(edge_string: &str) -> Result<Self> {
        let parts: Vec<&str> = (*edge_string
            .split('[')
            .collect::<Vec<&str>>()
            .get(0)
            .ok_or(anyhow!("Edge::from_str parsing error: No edge params."))?)
        .split("->")
        .map(|p| p.trim())
        .collect();

        Ok(Edge {
            parent: parts
                .get(0)
                .ok_or(anyhow!(
                    "Edge::from_str parsing error: Could not find first node index."
                ))?
                .to_string(),
            child: parts
                .get(1)
                .ok_or(anyhow!(
                    "Edge::from_str parsing error: Could not find second node index."
                ))?
                .to_string(),
            // weight: 1,
        })
    }
}
