(module
  (type (;0;) (func (param f32 f32) (result i32 f64 i32 f32 f32)))
  (type (;1;) (func (result f64 i32 f32 f32)))
  (type (;2;) (func (result f64 i32 f32 f32)))
  (type (;3;) (func (result f64 i32 f32 f32)))
  (type (;4;) (func (result f64 i32)))
  (type (;5;) (func))
  (type (;6;) (func (result f64 i32 f32 f32)))
  (type (;7;) (func (result f64 i32 f32 f32)))
  (type (;8;) (func (result f64 i32 f32 f32)))
  (type (;9;) (func (result f64 i32 f32 f32)))
  (type (;10;) (func (result f64 i32 f32 f32)))
  (type (;11;) (func (result f64 i32 f32 f32)))
  (type (;12;) (func (result f64 i32 f32 f32)))
  (type (;13;) (func (result f64 i32 f32 f32)))
  (type (;14;) (func (result f64 i32)))
  (type (;15;) (func))
  (type (;16;) (func (result f64 i32 f32 f32)))
  (type (;17;) (func (result f64 i32)))
  (table (;0;) 1 396 funcref)
  (global (;0;) f32 f32.const 0x1.82a85ap+15 (;=49492.176;))
  (global (;1;) (mut i32) i32.const 1515869510)
  (global (;2;) (mut i32) i32.const 65544519)
  (global (;3;) (mut i32) i32.const 1000)
  (export "UZZ-TAG!" (func 0))
  (export "2" (table 0))
  (export "3" (global 0))
  (export "4" (global 1))
  (export "5" (global 2))
  (func (;0;) (export "main") (type 16) (result f64 i32 f32 f32)
    (local i32 f64 i32 f32 f32 f32 f32 f64 f64 f64)
    global.get 3
    i32.eqz
    if ;; label = @1
      unreachable
    end
    global.get 3
    i32.const 1
    i32.sub
    global.set 3
    local.get 1
    local.get 0
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    i32.trunc_sat_f32_u
    i32.clz
    f32.const nan:0x7fffff (;=NaN;)
    f32.ceil
    local.tee 5
    f32.const nan (;=NaN;)
    local.get 5
    local.get 5
    f32.eq
    select
    local.tee 6
    local.get 6
    f32.ne
    local.get 6
    f32.const inf (;=inf;)
    f32.eq
    local.get 6
    f32.const -inf (;=-inf;)
    f32.eq
    i32.or
    i32.or
    if ;; label = @1
      f32.const 0x0p+0 (;=0;)
      local.set 6
    end
    local.get 6
    f32.const -0x1p+63 (;=-9223372000000000000;)
    f32.lt
    if ;; label = @1
      f32.const -0x1p+63 (;=-9223372000000000000;)
      local.set 6
    end
    local.get 6
    f32.const 0x1.fffffep+62 (;=9223371500000000000;)
    f32.gt
    if ;; label = @1
      f32.const 0x1.fffffep+62 (;=9223371500000000000;)
      local.set 6
    end
    local.get 6
    i64.trunc_f32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 2
    f64.convert_i32_s
    f64.sub
    local.tee 7
    f64.const nan (;=NaN;)
    local.get 7
    local.get 7
    f64.eq
    select
    f64.neg
    f64.nearest
    local.tee 8
    f64.const nan (;=NaN;)
    local.get 8
    local.get 8
    f64.eq
    select
    f64.sqrt
    local.tee 9
    f64.const nan (;=NaN;)
    local.get 9
    local.get 9
    f64.eq
    select
    f64.lt
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    local.get 0
    local.tee 0
    f64.convert_i32_s
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    f32.const 0x0p+0 (;=0;)
    f32.const 0x0p+0 (;=0;)
  )
  (data (;0;) "")
  (data (;1;) "")
)

(assert_return (invoke "main") (f64.const 0) (i32.const 0) (f32.const 0) (f32.const 0))
