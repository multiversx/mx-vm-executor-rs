/// Specifies which set of opcodes should be used by the VM.
///
/// `V1` is the legacy opcode set that does **not** include support for bulk
/// memory operations. `V2` extends `V1` by adding bulk memory instructions,
/// such as `MemoryCopy` and `MemoryFill`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcodeVersion {
    /// Legacy opcode set without bulk memory support.
    ///
    /// Use this for modules or environments that were compiled or designed
    /// before bulk memory operations were introduced, or when compatibility
    /// with older tooling is required.
    V1,
    /// Opcode set with bulk memory support.
    ///
    /// Use this for modules that rely on bulk memory operations like
    /// `MemoryCopy` and `MemoryFill`, or when targeting newer runtimes that
    /// support these instructions.
    ///
    /// Note: `V2` does **not** add support for `memory.init` and `data.drop`.
    V2,
}
impl OpcodeVersion {
    /// Converts a numeric opcode version identifier into an `OpcodeVersion`.
    ///
    /// * `0` maps to [`OpcodeVersion::V1`] (legacy, no bulk memory support).
    /// * `1` maps to [`OpcodeVersion::V2`] (with bulk memory operations).
    ///
    /// Any other value returns `None`. This is typically used when decoding
    /// opcode version information from serialized formats or external
    /// configuration that represent the version as an integer.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(OpcodeVersion::V1),
            1 => Some(OpcodeVersion::V2),
            _ => None,
        }
    }
}
