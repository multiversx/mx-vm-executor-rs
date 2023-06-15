# multiversx-chain-vm-executor

The MultiversX VM is composed of two parts:
- A high-level VM, that connects to the protocol and manages the environment of the smart contract execution
- A low-level component, called "executor", whose sole purpose is running WebAssembly code, as given. The only current implementation of this component is a wrapper around Wasmer 2.2.

This crate contains the Rust implementation of the interface between the two VM layers.

There are two parts of this interface:
- The one going "forwards", from high-level, to low-level, used for starting execution: `Executor`, `Instance`.
- The one going "backwards", from low-level, to high-level, used by the executor to request data: `VMHooks`.
 