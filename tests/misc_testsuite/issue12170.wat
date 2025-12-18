(module
  (func (export "hi") (result v128)
    (local $i v128)
    v128.const i64x2 0xfa2675c080000000 0xe8a433230a7479e5
    local.set $i
    local.get $i
    local.get $i
    local.get $i
    i32x4.min_s
    i32x4.lt_s
  )

)
