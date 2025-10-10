(module $demo.wasm
  (type (;0;) (func))
  (type (;1;) (func (param i32)))
  (type (;2;) (func (param i32 i32) (result i32)))
  (type (;3;) (func (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $__wasi_proc_exit (type 1)))
  (func $__wasm_call_ctors (type 0)
    call 9)
  (func $add (type 2) (param i32 i32) (result i32)
    (local i32)
    global.get 0
    i32.const 16
    i32.sub
    local.set 2
    local.get 2
    local.get 0
    i32.store offset=12
    local.get 2
    local.get 1
    i32.store offset=8
    local.get 2
    i32.load offset=12
    local.get 2
    i32.load offset=8
    i32.add
    return)
  (func $__original_main (type 3) (result i32)
    (local i32 i32)
    global.get 0
    i32.const 16
    i32.sub
    local.set 0
    local.get 0
    global.set 0
    local.get 0
    i32.const 0
    i32.store offset=12
    local.get 0
    i32.const 2
    i32.const 3
    call 2
    i32.store offset=8
    local.get 0
    i32.load offset=8
    local.set 1
    local.get 0
    i32.const 16
    i32.add
    global.set 0
    local.get 1
    return)
  (func $_start (type 0)
    block  ;; label = @1
      i32.const 1
      i32.eqz
      br_if 0 (;@1;)
      call 1
    end
    call 3
    call 7
    unreachable)
  (func $dummy (type 0))
  (func $libc_exit_fini (type 0)
    (local i32)
    i32.const 0
    local.set 0
    block  ;; label = @1
      i32.const 0
      i32.const 0
      i32.le_u
      br_if 0 (;@1;)
      loop  ;; label = @2
        local.get 0
        i32.const -4
        i32.add
        local.tee 0
        i32.load
        call_indirect (type 0)
        local.get 0
        i32.const 0
        i32.gt_u
        br_if 0 (;@2;)
      end
    end
    call 5)
  (func $exit (type 1) (param i32)
    call 5
    call 6
    call 5
    local.get 0
    call 8
    unreachable)
  (func $_Exit (type 1) (param i32)
    local.get 0
    call 0
    unreachable)
  (func $emscripten_stack_init (type 0)
    i32.const 65536
    global.set 2
    i32.const 0
    i32.const 15
    i32.add
    i32.const -16
    i32.and
    global.set 1)
  (func $emscripten_stack_get_free (type 3) (result i32)
    global.get 0
    global.get 1
    i32.sub)
  (func $emscripten_stack_get_base (type 3) (result i32)
    global.get 2)
  (func $emscripten_stack_get_end (type 3) (result i32)
    global.get 1)
  (func $_emscripten_stack_restore (type 1) (param i32)
    local.get 0
    global.set 0)
  (func $emscripten_stack_get_current (type 3) (result i32)
    global.get 0)
  (table (;0;) 2 2 funcref)
  (memory (;0;) 257 257)
  (global $__stack_pointer (mut i32) (i32.const 65536))
  (global $__stack_end (mut i32) (i32.const 0))
  (global $__stack_base (mut i32) (i32.const 0))
  (export "memory" (memory 0))
  (export "__indirect_function_table" (table 0))
  (export "_start" (func 4))
  (export "emscripten_stack_init" (func 9))
  (export "emscripten_stack_get_free" (func 10))
  (export "emscripten_stack_get_base" (func 11))
  (export "emscripten_stack_get_end" (func 12))
  (export "_emscripten_stack_restore" (func 13))
  (export "emscripten_stack_get_current" (func 14))
  (elem (;0;) (i32.const 1) func 1))
