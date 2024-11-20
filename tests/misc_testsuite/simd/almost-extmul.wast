;;! simd = true

;; regression test from #3337, there's a multiplication that sort of
;; looks like an extmul and codegen shouldn't pattern match too much
(module
  (type (;0;) (func))
  (func (;0;) (type 0)
    v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000
    i64x2.extend_low_i32x4_u
    v128.const i32x4 0x00000000 0x00000000 0x00000000 0x00000000
    i64x2.mul
    i32x4.all_true
    i64.load offset=1 align=1
    drop
    unreachable)
  (func (;1;) (type 0)
    nop)
  (memory (;0;) 1 1))
