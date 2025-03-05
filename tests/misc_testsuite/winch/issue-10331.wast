;;! simd = true

;; See https://github.com/bytecodealliance/wasmtime/issues/10331

(module
  (func (export "test") (result v128)
    v128.const i8x16 0 128 0 0 0 0 0 0 0 0 0 0 0 0 0 0
    call 1
  )
  (func (param v128) (result v128)
    local.get 0
    i16x8.extadd_pairwise_i8x16_s
  )
)

(assert_return (invoke "test") (v128.const i16x8 65408 0 0 0 0 0 0 0))
