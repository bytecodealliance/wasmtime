(; See issue https://github.com/bytecodealliance/wasmtime/issues/3173. ;)

(module
  (func (export "select_v128") (result v128)
    v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000
    v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000
    i32.const 0
    select))

(assert_return (invoke "select_v128") (v128.const i32x4 0 0 0 0))
