// Code generated by vmhooks generator. DO NOT EDIT.

// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// !!!!!!!!!!!!!!!!!!!!!! AUTO-GENERATED FILE !!!!!!!!!!!!!!!!!!!!!!
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use multiversx_vm_executor::OpcodeCost;
use wasmer::wasmparser::Operator;

pub fn get_local_cost(opcode_cost: &OpcodeCost) -> u32 {
	opcode_cost.opcode_localallocate
}

pub fn get_opcode_cost(op: &Operator, opcode_cost: &OpcodeCost) -> Option<u32> {
    match op {
        Operator::Block { .. } => Some(opcode_cost.opcode_block),
        Operator::Br { .. } => Some(opcode_cost.opcode_br),
        Operator::BrIf { .. } => Some(opcode_cost.opcode_brif),
        Operator::BrTable { .. } => Some(opcode_cost.opcode_brtable),
        Operator::Call { .. } => Some(opcode_cost.opcode_call),
        Operator::CallIndirect { .. } => Some(opcode_cost.opcode_callindirect),
        Operator::Catch { .. } => Some(opcode_cost.opcode_catch),
        Operator::CatchAll { .. } => Some(opcode_cost.opcode_catchall),
        Operator::Delegate { .. } => Some(opcode_cost.opcode_delegate),
        Operator::Drop { .. } => Some(opcode_cost.opcode_drop),
        Operator::Else { .. } => Some(opcode_cost.opcode_else),
        Operator::End { .. } => Some(opcode_cost.opcode_end),
        Operator::GlobalGet { .. } => Some(opcode_cost.opcode_globalget),
        Operator::GlobalSet { .. } => Some(opcode_cost.opcode_globalset),
        Operator::I32Add { .. } => Some(opcode_cost.opcode_i32add),
        Operator::I32And { .. } => Some(opcode_cost.opcode_i32and),
        Operator::I32Clz { .. } => Some(opcode_cost.opcode_i32clz),
        Operator::I32Const { .. } => Some(opcode_cost.opcode_i32const),
        Operator::I32Ctz { .. } => Some(opcode_cost.opcode_i32ctz),
        Operator::I32DivS { .. } => Some(opcode_cost.opcode_i32divs),
        Operator::I32DivU { .. } => Some(opcode_cost.opcode_i32divu),
        Operator::I32Eq { .. } => Some(opcode_cost.opcode_i32eq),
        Operator::I32Eqz { .. } => Some(opcode_cost.opcode_i32eqz),
        Operator::I32Extend16S { .. } => Some(opcode_cost.opcode_i32extend16s),
        Operator::I32Extend8S { .. } => Some(opcode_cost.opcode_i32extend8s),
        Operator::I32GeS { .. } => Some(opcode_cost.opcode_i32ges),
        Operator::I32GeU { .. } => Some(opcode_cost.opcode_i32geu),
        Operator::I32GtS { .. } => Some(opcode_cost.opcode_i32gts),
        Operator::I32GtU { .. } => Some(opcode_cost.opcode_i32gtu),
        Operator::I32LeS { .. } => Some(opcode_cost.opcode_i32les),
        Operator::I32LeU { .. } => Some(opcode_cost.opcode_i32leu),
        Operator::I32Load { .. } => Some(opcode_cost.opcode_i32load),
        Operator::I32Load16S { .. } => Some(opcode_cost.opcode_i32load16s),
        Operator::I32Load16U { .. } => Some(opcode_cost.opcode_i32load16u),
        Operator::I32Load8S { .. } => Some(opcode_cost.opcode_i32load8s),
        Operator::I32Load8U { .. } => Some(opcode_cost.opcode_i32load8u),
        Operator::I32LtS { .. } => Some(opcode_cost.opcode_i32lts),
        Operator::I32LtU { .. } => Some(opcode_cost.opcode_i32ltu),
        Operator::I32Mul { .. } => Some(opcode_cost.opcode_i32mul),
        Operator::I32Ne { .. } => Some(opcode_cost.opcode_i32ne),
        Operator::I32Or { .. } => Some(opcode_cost.opcode_i32or),
        Operator::I32Popcnt { .. } => Some(opcode_cost.opcode_i32popcnt),
        Operator::I32RemS { .. } => Some(opcode_cost.opcode_i32rems),
        Operator::I32RemU { .. } => Some(opcode_cost.opcode_i32remu),
        Operator::I32Rotl { .. } => Some(opcode_cost.opcode_i32rotl),
        Operator::I32Rotr { .. } => Some(opcode_cost.opcode_i32rotr),
        Operator::I32Shl { .. } => Some(opcode_cost.opcode_i32shl),
        Operator::I32ShrS { .. } => Some(opcode_cost.opcode_i32shrs),
        Operator::I32ShrU { .. } => Some(opcode_cost.opcode_i32shru),
        Operator::I32Store { .. } => Some(opcode_cost.opcode_i32store),
        Operator::I32Store16 { .. } => Some(opcode_cost.opcode_i32store16),
        Operator::I32Store8 { .. } => Some(opcode_cost.opcode_i32store8),
        Operator::I32Sub { .. } => Some(opcode_cost.opcode_i32sub),
        Operator::I32WrapI64 { .. } => Some(opcode_cost.opcode_i32wrapi64),
        Operator::I32Xor { .. } => Some(opcode_cost.opcode_i32xor),
        Operator::I64Add { .. } => Some(opcode_cost.opcode_i64add),
        Operator::I64And { .. } => Some(opcode_cost.opcode_i64and),
        Operator::I64Clz { .. } => Some(opcode_cost.opcode_i64clz),
        Operator::I64Const { .. } => Some(opcode_cost.opcode_i64const),
        Operator::I64Ctz { .. } => Some(opcode_cost.opcode_i64ctz),
        Operator::I64DivS { .. } => Some(opcode_cost.opcode_i64divs),
        Operator::I64DivU { .. } => Some(opcode_cost.opcode_i64divu),
        Operator::I64Eq { .. } => Some(opcode_cost.opcode_i64eq),
        Operator::I64Eqz { .. } => Some(opcode_cost.opcode_i64eqz),
        Operator::I64Extend16S { .. } => Some(opcode_cost.opcode_i64extend16s),
        Operator::I64Extend32S { .. } => Some(opcode_cost.opcode_i64extend32s),
        Operator::I64Extend8S { .. } => Some(opcode_cost.opcode_i64extend8s),
        Operator::I64ExtendI32S { .. } => Some(opcode_cost.opcode_i64extendi32s),
        Operator::I64ExtendI32U { .. } => Some(opcode_cost.opcode_i64extendi32u),
        Operator::I64GeS { .. } => Some(opcode_cost.opcode_i64ges),
        Operator::I64GeU { .. } => Some(opcode_cost.opcode_i64geu),
        Operator::I64GtS { .. } => Some(opcode_cost.opcode_i64gts),
        Operator::I64GtU { .. } => Some(opcode_cost.opcode_i64gtu),
        Operator::I64LeS { .. } => Some(opcode_cost.opcode_i64les),
        Operator::I64LeU { .. } => Some(opcode_cost.opcode_i64leu),
        Operator::I64Load { .. } => Some(opcode_cost.opcode_i64load),
        Operator::I64Load16S { .. } => Some(opcode_cost.opcode_i64load16s),
        Operator::I64Load16U { .. } => Some(opcode_cost.opcode_i64load16u),
        Operator::I64Load32S { .. } => Some(opcode_cost.opcode_i64load32s),
        Operator::I64Load32U { .. } => Some(opcode_cost.opcode_i64load32u),
        Operator::I64Load8S { .. } => Some(opcode_cost.opcode_i64load8s),
        Operator::I64Load8U { .. } => Some(opcode_cost.opcode_i64load8u),
        Operator::I64LtS { .. } => Some(opcode_cost.opcode_i64lts),
        Operator::I64LtU { .. } => Some(opcode_cost.opcode_i64ltu),
        Operator::I64Mul { .. } => Some(opcode_cost.opcode_i64mul),
        Operator::I64Ne { .. } => Some(opcode_cost.opcode_i64ne),
        Operator::I64Or { .. } => Some(opcode_cost.opcode_i64or),
        Operator::I64Popcnt { .. } => Some(opcode_cost.opcode_i64popcnt),
        Operator::I64RemS { .. } => Some(opcode_cost.opcode_i64rems),
        Operator::I64RemU { .. } => Some(opcode_cost.opcode_i64remu),
        Operator::I64Rotl { .. } => Some(opcode_cost.opcode_i64rotl),
        Operator::I64Rotr { .. } => Some(opcode_cost.opcode_i64rotr),
        Operator::I64Shl { .. } => Some(opcode_cost.opcode_i64shl),
        Operator::I64ShrS { .. } => Some(opcode_cost.opcode_i64shrs),
        Operator::I64ShrU { .. } => Some(opcode_cost.opcode_i64shru),
        Operator::I64Store { .. } => Some(opcode_cost.opcode_i64store),
        Operator::I64Store16 { .. } => Some(opcode_cost.opcode_i64store16),
        Operator::I64Store32 { .. } => Some(opcode_cost.opcode_i64store32),
        Operator::I64Store8 { .. } => Some(opcode_cost.opcode_i64store8),
        Operator::I64Sub { .. } => Some(opcode_cost.opcode_i64sub),
        Operator::I64Xor { .. } => Some(opcode_cost.opcode_i64xor),
        Operator::If { .. } => Some(opcode_cost.opcode_if),
        Operator::LocalGet { .. } => Some(opcode_cost.opcode_localget),
        Operator::LocalSet { .. } => Some(opcode_cost.opcode_localset),
        Operator::LocalTee { .. } => Some(opcode_cost.opcode_localtee),
        Operator::Loop { .. } => Some(opcode_cost.opcode_loop),
        Operator::MemoryGrow { .. } => Some(opcode_cost.opcode_memorygrow),
        Operator::MemorySize { .. } => Some(opcode_cost.opcode_memorysize),
        Operator::Nop { .. } => Some(opcode_cost.opcode_nop),
        Operator::RefFunc { .. } => Some(opcode_cost.opcode_reffunc),
        Operator::RefIsNull { .. } => Some(opcode_cost.opcode_refisnull),
        Operator::RefNull { .. } => Some(opcode_cost.opcode_refnull),
        Operator::Rethrow { .. } => Some(opcode_cost.opcode_rethrow),
        Operator::Return { .. } => Some(opcode_cost.opcode_return),
        Operator::ReturnCall { .. } => Some(opcode_cost.opcode_returncall),
        Operator::ReturnCallIndirect { .. } => Some(opcode_cost.opcode_returncallindirect),
        Operator::Select { .. } => Some(opcode_cost.opcode_select),
        Operator::TableGet { .. } => Some(opcode_cost.opcode_tableget),
        Operator::TableGrow { .. } => Some(opcode_cost.opcode_tablegrow),
        Operator::TableInit { .. } => Some(opcode_cost.opcode_tableinit),
        Operator::TableSet { .. } => Some(opcode_cost.opcode_tableset),
        Operator::TableSize { .. } => Some(opcode_cost.opcode_tablesize),
        Operator::Throw { .. } => Some(opcode_cost.opcode_throw),
        Operator::Try { .. } => Some(opcode_cost.opcode_try),
        Operator::TypedSelect { .. } => Some(opcode_cost.opcode_typedselect),
        Operator::Unreachable { .. } => Some(opcode_cost.opcode_unreachable),
        Operator::Unwind { .. } => Some(opcode_cost.opcode_unwind),
		_ => None,
    }
}
