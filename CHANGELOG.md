# Change Log

This file contains a centralizes a trace of all published crate versions, with their changes in short.

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
