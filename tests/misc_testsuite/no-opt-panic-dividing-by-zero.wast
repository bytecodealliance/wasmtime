(module
  (func (result i32)
    (local i64 i32 i32)
    i32.const 1
    i32.clz
    i32.clz
    i32.popcnt
    i64.extend_i32_s
    local.get 0
    i64.mul
    local.get 0
    i64.mul
    i32.wrap_i64
    i64.extend_i32_s
    local.get 1
    local.get 0
    i32.const 1
    i32.clz
    i32.clz
    i32.popcnt
    i64.extend_i32_s
    i64.mul
    i64.const 0
    i64.eq
    i32.rotl
    i64.extend_i32_s
    i64.mul
    i32.wrap_i64
  )
)
