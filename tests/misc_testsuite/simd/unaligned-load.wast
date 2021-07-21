(; See discussion at https://github.com/bytecodealliance/wasmtime/issues/2943 ;)
(module
  (memory 1)
  (data (i32.const 1) "\01\00\00\00\01\00\00\00")

  (func $unaligned_load (export "unaligned_load") (result v128)
    v128.const i32x4 0 0 1 1
    i32.const 1
    v128.load
    v128.xor)
)

(assert_return (invoke "unaligned_load") (v128.const i32x4 1 1 1 1))
