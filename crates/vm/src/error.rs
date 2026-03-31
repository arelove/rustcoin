use thiserror::Error;

/// Errors that can occur during WASM contract execution.
#[derive(Debug, Error)]
pub enum VmError {
    /// Failed to initialize the WASM engine.
    #[error("vm init error: {0}")]
    Init(String),
    /// Failed to compile the WASM bytecode.
    #[error("compile error: {0}")]
    Compile(String),
    /// Failed to instantiate the WASM module.
    #[error("instantiate error: {0}")]
    Instantiate(String),
    /// An error occurred during contract execution.
    #[error("execution error: {0}")]
    Execution(String),
    /// The contract exceeded its gas limit.
    #[error("out of gas (limit: {limit}, used: {used})")]
    OutOfGas {
        /// The gas limit that was set for this execution.
        limit: u64,
        /// The amount of gas consumed before running out.
        used: u64,
    },
    /// No contract found at the given address.
    #[error("contract not found: {0}")]
    ContractNotFound(String),
    /// The contract's ABI is invalid or incompatible.
    #[error("invalid ABI: {0}")]
    InvalidAbi(String),
}
