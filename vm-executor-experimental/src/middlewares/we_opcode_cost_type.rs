/// What executing one WASM operator costs.
///
/// Not to be confused with `OpcodeCost`, which is the full cost table, as configured by the
/// gas schedule. This is the cost of one occurrence of one operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    /// The operator is not allowed by the configured opcode version, so it has no cost.
    ///
    /// Modules that contain it are rejected at compile time.
    Illegal,

    /// The operator is charged once, no matter what its operands are.
    Base(u32),

    /// A bulk memory operator (`memory.copy`, `memory.fill`), charged once for the operation
    /// itself, plus once for every byte it copies or fills.
    ///
    /// The base cost is what keeps processing zero bytes from being free.
    BulkMemory { base: u32, per_byte: u32 },
}
