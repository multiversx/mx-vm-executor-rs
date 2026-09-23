(module
  (memory $mem 1)
  (func $copyZero (type $void)
    (memory.copy
      (i32.const 0)     ;; destination address
      (i32.const 64)    ;; source address
      (i32.const 0)     ;; size of memory region in bytes
    )
  )
  (func $copyTen (type $void)
    (memory.copy
      (i32.const 0)     ;; destination address
      (i32.const 64)    ;; source address
      (i32.const 10)    ;; size of memory region in bytes
    )
  )
  (func $fillZero (type $void)
    (memory.fill
      (i32.const 0)     ;; destination address
      (i32.const 7)     ;; value to fill with
      (i32.const 0)     ;; size of memory region in bytes
    )
  )
  (func $fillTen (type $void)
    (memory.fill
      (i32.const 0)     ;; destination address
      (i32.const 7)     ;; value to fill with
      (i32.const 10)    ;; size of memory region in bytes
    )
  )
  ;; same as $copyTen, but with a size that the compiler cannot see through,
  ;; to check that the size operand survives the metering injection unchanged
  (func $copyLocalTen (type $void) (local $size i32)
    (local.set $size (i32.const 10))
    (memory.copy
      (i32.const 0)     ;; destination address
      (i32.const 64)    ;; source address
      (local.get $size) ;; size of memory region in bytes
    )
  )
  (type $void (func))
  (export "memory" (memory $mem))
  (export "copyZero" (func $copyZero))
  (export "copyTen" (func $copyTen))
  (export "fillZero" (func $fillZero))
  (export "fillTen" (func $fillTen))
  (export "copyLocalTen" (func $copyLocalTen))
)
