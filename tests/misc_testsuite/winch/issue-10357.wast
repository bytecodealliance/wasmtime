;;! simd = true

;; See https://github.com/bytecodealliance/wasmtime/issues/10357

(module
  (func (export "test") (result i64 v128 v128)
    i32.const 0
    v128.const i32x4 0x00000000 0x00000000 0x68732efe 0x74727473
    i64.const 1
    call 1
  )

  (func (param i32 v128 i64) (result i64 v128 v128)
    i64.const 0
    local.get 1
    v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000
  )
)

(assert_return (invoke "test") (i64.const 0) (v128.const i32x4 0x00000000 0x00000000 0x68732efe 0x74727473) (v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000))
