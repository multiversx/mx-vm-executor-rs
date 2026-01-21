use crate::{OpcodeCost, OpcodeVersion};

#[derive(Debug, Clone)]
pub struct OpcodeConfig {
    pub opcode_version: OpcodeVersion,
    pub opcode_cost: OpcodeCost,
}

impl Default for OpcodeConfig {
    fn default() -> Self {
        Self {
            opcode_version: OpcodeVersion::V1,
            opcode_cost: OpcodeCost::default(),
        }
    }
}
