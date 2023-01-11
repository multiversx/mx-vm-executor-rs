use loupe::{MemoryUsage, MemoryUsageTracker};
use std::fs::File;
use std::io::Write;
use std::mem;
use wasmer::wasmparser::Operator;
use wasmer::{
    FunctionMiddleware, LocalFunctionIndex, MiddlewareError, MiddlewareReaderState,
    ModuleMiddleware,
};
use wasmer_types::ModuleInfo;

const OPCODE_TRACE_PATH: &str = "opcode.trace2";

#[derive(Debug)]
pub(crate) struct OpcodeTracer {}

impl OpcodeTracer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

unsafe impl Send for OpcodeTracer {}
unsafe impl Sync for OpcodeTracer {}

impl MemoryUsage for OpcodeTracer {
    fn size_of_val(&self, _tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
    }
}

impl ModuleMiddleware for OpcodeTracer {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionOpcodeTracer {
            output_file: File::create(OPCODE_TRACE_PATH).unwrap(),
        })
    }

    fn transform_module_info(&self, _module_info: &mut ModuleInfo) {}
}

#[derive(Debug)]
struct FunctionOpcodeTracer {
    output_file: File,
}

impl FunctionOpcodeTracer {
    fn trace_opcode<'b>(&mut self, operator: &Operator<'b>) -> Result<(), MiddlewareError> {
        let result = writeln!(self.output_file, "\t{:?}", operator);
        if let Err(error) = result {
            return Err(MiddlewareError::new(
                "opcode_trace_middleware",
                error.to_string(),
            ));
        }

        Ok(())
    }
}

impl FunctionMiddleware for FunctionOpcodeTracer {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        self.trace_opcode(&operator)?;

        state.push_operator(operator);

        Ok(())
    }
}
