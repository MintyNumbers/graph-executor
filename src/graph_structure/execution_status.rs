use anyhow::{anyhow, Error, Result};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, Copy)]
pub enum ExecutionStatus {
    /// The associated [`super::node::Node`]'s `execute()` method is finished.
    Executed,
    /// The associated [`super::node::Node`] currently runs its `execute()` method.
    Executing,
    /// The associated [`super::node::Node`]'s `execute()` method is ready to run;
    /// all its parent [`super::node::Node`]s have run their respective `execute()` methods.
    Executable,
    /// The associated [`super::node::Node`]'s `execute()` method is not ready to run;
    /// not all its parent [`super::node::Node`]s have run their respective `execute()` methods.
    NonExecutable,
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ExecutionStatus::Executed => "Executed",
                ExecutionStatus::Executing => "Executing",
                ExecutionStatus::Executable => "Executable",
                ExecutionStatus::NonExecutable => "NonExecutable",
            }
        )
    }
}

impl FromStr for ExecutionStatus {
    type Err = Error;
    /// Parses [`ExecutionStatus`] from a string like: "Executed".
    ///
    /// The following two [`ExecutionStatus`] are identical:
    /// ```
    /// let execution_status_from_str = ExecutionStatus::from_str("Executed").unwrap();
    /// let execution_status_direct = ExecutionStatus::Executed;
    /// ```
    fn from_str(execution_status_string: &str) -> Result<Self> {
        match execution_status_string {
            "Executed" => Ok(ExecutionStatus::Executed),
            "Executing" => Ok(ExecutionStatus::Executing),
            "Executable" => Ok(ExecutionStatus::Executable),
            "NonExecutable" => Ok(ExecutionStatus::NonExecutable),
            _ => Err(anyhow!(
                "ExecutionStatus::from_str parsing error: Invalid execution status."
            )),
        }
    }
}
