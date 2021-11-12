(; See issue https://github.com/bytecodealliance/wasmtime/issues/3327 ;)

(module
  (func $v128_not (export "v128_not") (result v128)
    v128.const f32x4 0 0 0 0
    f32x4.abs
    v128.not)
)

(assert_return (invoke "v128_not") (v128.const i32x4 -1 -1 -1 -1))