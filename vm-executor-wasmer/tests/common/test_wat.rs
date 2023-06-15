pub const EMPTY_SC_WAT: &str = r#"
(module
    (type (;0;) (func))
    (type (;1;) (func (param i32 i32)))
    (type (;2;) (func (result i32)))
    (import "env" "getNumArguments" (func (;0;) (type 2)))
    (import "env" "signalError" (func (;1;) (type 1)))
    (import "env" "checkNoPayment" (func (;2;) (type 0)))
    (func (;3;) (type 0)
      call 2
      call 0
      if  ;; label = @1
        i32.const 1048576
        i32.const 25
        call 1
        unreachable
      end)
    (func (;4;) (type 0)
      nop)
    (memory (;0;) 17)
    (global (;0;) i32 (i32.const 1048601))
    (global (;1;) i32 (i32.const 1048608))
    (export "memory" (memory 0))
    (export "init" (func 3))
    (export "callBack" (func 4))
    (export "__data_end" (global 0))
    (export "__heap_base" (global 1))
    (data (;0;) (i32.const 1048576) "wrong number of arguments"))
"#;

/// The debug version, for an easier read.
pub const ADDER_WAT: &str = r#"
(module
    (type (;0;) (func (param i32 i32)))
    (type (;1;) (func (result i32)))
    (type (;2;) (func (param i32 i32) (result i32)))
    (type (;3;) (func (param i32 i32 i32) (result i32)))
    (type (;4;) (func))
    (type (;5;) (func (param i32 i32 i32)))
    (type (;6;) (func (param i32)))
    (type (;7;) (func (param i32) (result i32)))
    (import "env" "bigIntGetUnsignedArgument" (func $bigIntGetUnsignedArgument (type 0)))
    (import "env" "getNumArguments" (func $getNumArguments (type 1)))
    (import "env" "signalError" (func $signalError (type 0)))
    (import "env" "mBufferStorageLoad" (func $mBufferStorageLoad (type 2)))
    (import "env" "mBufferToBigIntUnsigned" (func $mBufferToBigIntUnsigned (type 2)))
    (import "env" "mBufferFromBigIntUnsigned" (func $mBufferFromBigIntUnsigned (type 2)))
    (import "env" "mBufferStorageStore" (func $mBufferStorageStore (type 2)))
    (import "env" "mBufferSetBytes" (func $mBufferSetBytes (type 3)))
    (import "env" "checkNoPayment" (func $checkNoPayment (type 4)))
    (import "env" "bigIntAdd" (func $bigIntAdd (type 5)))
    (import "env" "bigIntFinishUnsigned" (func $bigIntFinishUnsigned (type 6)))
    (func $_ZN11elrond_wasm2io16arg_nested_tuple15load_single_arg17hc323659743ed9ee4E (type 1) (result i32)
      (local i32)
      i32.const 0
      call $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E
      local.tee 0
      call $bigIntGetUnsignedArgument
      local.get 0)
    (func $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E (type 1) (result i32)
      (local i32)
      i32.const 0
      i32.const 0
      i32.load offset=1048604
      i32.const -1
      i32.add
      local.tee 0
      i32.store offset=1048604
      local.get 0)
    (func $_ZN11elrond_wasm2io16arg_nested_tuple22check_num_arguments_eq17h848f7f930a70d303E (type 6) (param i32)
      block  ;; label = @1
        call $getNumArguments
        local.get 0
        i32.ne
        br_if 0 (;@1;)
        return
      end
      i32.const 1048576
      i32.const 25
      call $signalError
      unreachable)
    (func $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3get17h25bb7558615ec585E (type 7) (param i32) (result i32)
      (local i32)
      local.get 0
      call $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E
      local.tee 1
      call $mBufferStorageLoad
      drop
      local.get 1
      call $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E
      local.tee 0
      call $mBufferToBigIntUnsigned
      drop
      local.get 0)
    (func $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3set17h646573d1e8aedfb8E (type 0) (param i32 i32)
      (local i32)
      call $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E
      local.tee 2
      local.get 1
      call $mBufferFromBigIntUnsigned
      drop
      local.get 0
      local.get 2
      call $mBufferStorageStore
      drop)
    (func $_ZN34_$LT$C$u20$as$u20$adder..Adder$GT$3sum17h4be7469c9e0e8813E (type 1) (result i32)
      (local i32)
      call $_ZN16elrond_wasm_node3api13managed_types19static_var_api_node11next_handle17hdafd854c2ac6d869E
      local.tee 0
      i32.const 1048601
      i32.const 3
      call $mBufferSetBytes
      drop
      local.get 0)
    (func $init (type 4)
      (local i32)
      call $checkNoPayment
      i32.const 1
      call $_ZN11elrond_wasm2io16arg_nested_tuple22check_num_arguments_eq17h848f7f930a70d303E
      call $_ZN11elrond_wasm2io16arg_nested_tuple15load_single_arg17hc323659743ed9ee4E
      local.set 0
      call $_ZN34_$LT$C$u20$as$u20$adder..Adder$GT$3sum17h4be7469c9e0e8813E
      local.get 0
      call $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3set17h646573d1e8aedfb8E)
    (func $add (type 4)
      (local i32 i32 i32)
      call $checkNoPayment
      i32.const 1
      call $_ZN11elrond_wasm2io16arg_nested_tuple22check_num_arguments_eq17h848f7f930a70d303E
      call $_ZN11elrond_wasm2io16arg_nested_tuple15load_single_arg17hc323659743ed9ee4E
      local.set 0
      call $_ZN34_$LT$C$u20$as$u20$adder..Adder$GT$3sum17h4be7469c9e0e8813E
      local.tee 1
      call $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3get17h25bb7558615ec585E
      local.tee 2
      local.get 2
      local.get 0
      call $bigIntAdd
      local.get 1
      local.get 2
      call $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3set17h646573d1e8aedfb8E)
    (func $getSum (type 4)
      call $checkNoPayment
      i32.const 0
      call $_ZN11elrond_wasm2io16arg_nested_tuple22check_num_arguments_eq17h848f7f930a70d303E
      call $_ZN34_$LT$C$u20$as$u20$adder..Adder$GT$3sum17h4be7469c9e0e8813E
      call $_ZN11elrond_wasm7storage7mappers19single_value_mapper31SingleValueMapper$LT$SA$C$T$GT$3get17h25bb7558615ec585E
      call $bigIntFinishUnsigned)
    (func $callBack (type 4))
    (table (;0;) 1 1 funcref)
    (memory (;0;) 17)
    (global $__stack_pointer (mut i32) (i32.const 1048576))
    (global (;1;) i32 (i32.const 1048608))
    (global (;2;) i32 (i32.const 1048608))
    (export "memory" (memory 0))
    (export "init" (func $init))
    (export "add" (func $add))
    (export "getSum" (func $getSum))
    (export "callBack" (func $callBack))
    (export "__data_end" (global 1))
    (export "__heap_base" (global 2))
    (data $.rodata (i32.const 1048576) "wrong number of argumentssum")
    (data $.data (i32.const 1048604) "\9c\ff\ff\ff"))
"#;
