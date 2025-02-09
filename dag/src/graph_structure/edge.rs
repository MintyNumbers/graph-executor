use anyhow::{anyhow, Error, Result};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Edge {
    pub nodes: (usize, usize),
    // pub weight: i32,
}

impl Edge {
    pub fn new(nodes: (usize, usize) /* , weight: i32 */) -> Self {
        Edge {
            nodes: (nodes.0, nodes.1),
            // weight: weight,
        }
    }
}

impl FromStr for Edge {
    type Err = Error;
    /// Parses `Edge` from a string like: "0 -> 1 [ ]"
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
            nodes: (
                usize::from_str(*(parts.get(0).ok_or(anyhow!("Edge::from_str parsing error: Could not find first node index."))?))?,
                usize::from_str(*(parts.get(1).ok_or(anyhow!("Edge::from_str parsing error: Could not find second node index."))?))?,
            ),
            // weight: 1,
        })
    }
}
