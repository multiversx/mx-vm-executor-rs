use multiversx_chain_vm_executor::{GasSchedule, OpcodeCost};
use std::path::Path;

const GAS_SCHEDULE_V8: &str = "gasScheduleV8.toml";

#[test]
fn deserialization_test_from_file() {
    let file_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schedules")
        .join(GAS_SCHEDULE_V8);

    let result = OpcodeCost::from_file(file_path);
    assert!(result.is_ok(), "Failed to deserialize: {:?}", result.err());

    let opcode_cost = result.unwrap();

    assert_eq!(opcode_cost.opcode_unreachable, 5);
    assert_eq!(opcode_cost.opcode_nop, 5);
    assert_eq!(opcode_cost.opcode_block, 5);
}

#[test]
fn new_struct_with_gas_schedule_test() {
    let opcode_cost = OpcodeCost::new(GasSchedule::V8);

    assert_eq!(opcode_cost.opcode_unreachable, 5);
    assert_eq!(opcode_cost.opcode_nop, 5);
    assert_eq!(opcode_cost.opcode_block, 5);
}
