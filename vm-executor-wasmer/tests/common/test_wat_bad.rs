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

// v128.store is not supported in wasmer
pub const UNSUPPORTED_OPERATIONS: &str = r#"
(module
    (type (;0;) (func))
    (type (;1;) (func (param i32 i32)))
    (type (;2;) (func (result i32)))
    (type (;3;) (func (param v128 i32 v128)))
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
    (func $verifier_errors (type 3) (param v128 i32 v128)
      local.get 1
      local.get 0
      local.get 2
      i8x16.eq
      v128.store
    )
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