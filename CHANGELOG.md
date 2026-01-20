# Change Log

This file contains a centralizes a trace of all published crate versions, with their changes in short.

## [multiversx-chain-vm-executor 0.5.1] - 2026-01-20
- Upgraded to Wasmer v6.1.0.
- Fixed linker issue on Linux for Wasmer 2 (unreleased).
- Upgraded to edition 2024.
- Upgraded dependencies.

## [multiversx-chain-vm-executor 0.5.0] - 2025-07-03
- Barnard VM hooks.

## [multiversx-chain-vm-executor 0.4.0] - 2025-05-23
- Rust VM support:
    - New set of traits to work with the Rust VM;
    - Renamed old traits as `*Legacy`.
- Wasmer crate gets an additional adapter to the new interfaces (unreleased).
- New wasmer experimental integration crate, with Wasmer 6.

## [multiversx-chain-vm-executor 0.3.0] - 2024-11-14
- Spica release.
- EI v1.4 new hooks:
    - isReservedFunctionName
    - managedGetOriginalCallerAddr
    - managedGetRelayerAddr
    - managedMultiTransferESDTNFTExecuteByUser
    - managedVerifySecp256r1
    - managedVerifyBLSSignatureShare
    - managedVerifyBLSAggregatedSignature

## [multiversx-chain-vm-executor 0.2.0] - 2023-10-12
- New VM hook: `managedGetBackTransfers`.
- Memory fix.

## [multiversx-chain-vm-executor 0.1.0] - 2023-06-15
This is the initial official release of the VM executor interface. The purpose is for it to be used in the new smart contract debugger architecture.

It targets VM 1.5 and integrates the Wasmer 2.2 implementation.
