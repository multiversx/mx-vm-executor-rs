pub const BAD_INIT_PARAM: &str = r#"
(module
    (type $t0 (func (param i32)))
    (func $init (type $t0) (param $p0 i32))
    (memory $memory 2)
    (export "memory" (memory 0))
    (export "init" (func $init)))
"#;

pub const BAD_INIT_RESULT: &str = r#"
(module
    (type (;0;) (func (result i32)))
    (func (;0;) (type 0) (result i32)
      i32.const 42)
    (memory (;0;) 2)
    (export "memory" (memory 0))
    (export "init" (func 0)))
"#;
