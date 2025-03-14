use loupe::{MemoryUsage, MemoryUsageTracker};
use std::fs::{self, File};
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
        File::create(OPCODE_TRACE_PATH).unwrap();
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
        local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        let file = fs::OpenOptions::new()
            .append(true)
            .open(OPCODE_TRACE_PATH)
            .unwrap();

        Box::new(FunctionOpcodeTracer {
            output_file: file,
            local_function_index: local_function_index.as_u32(),
            counter: 0,
        })
    }

    fn transform_module_info(&self, _module_info: &mut ModuleInfo) {}
}

#[derive(Debug)]
struct FunctionOpcodeTracer {
    output_file: File,
    local_function_index: u32,
    counter: u32,
}

impl FunctionOpcodeTracer {
    fn trace_operator(&mut self, operator: &Operator) {
        self.output_file
            .write_all(
                format!(
                    "[fn: {:08b}({}), operator: {:08b}({})]\t{:?}\n",
                    self.local_function_index,
                    self.local_function_index,
                    self.counter,
                    self.counter,
                    operator
                )
                .as_bytes(),
            )
            .unwrap();
        self.counter += 1;
    }
}

impl FunctionMiddleware for FunctionOpcodeTracer {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        self.trace_operator(&operator);

        state.push_operator(operator);

        Ok(())
    }
}
