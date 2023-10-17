;;! target = "riscv64"
;;! compile = true
;;! settings = ["has_zbs", "opt_level=speed"]

(module
  (func (export "bclr32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (local.get 1)) (i32.const -1)))
  )
  (func (export "bclr64") (param i64 i64) (result i64)
    (i64.and (i64.xor (i64.shl (i64.const 1) (local.get 1)) (i64.const -1)) (local.get 0))
  )
  (func (export "bclri32_4") (param i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (i32.const 4)) (i32.const -1)))
  )
  (func (export "bclri32_20") (param i32) (result i32)
    (i32.and (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (i32.const -1)) (local.get 0))
  )
  (func (export "bclri64_4") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 4)) (i64.const -1)))
  )
  (func (export "bclri64_52") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 52)) (i64.const -1)))
  )

  (func (export "bext32_1") (param i32 i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_2") (param i32 i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (local.get 1)) (i32.const 1))
  )
  (func (export "bext32_3") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext32_4") (param i32 i32) (result i32)
    (i32.and (i32.const 1) (i32.shr_s (local.get 0) (local.get 1)))
  )
  (func (export "bext64_1") (param i64 i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_2") (param i64 i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (local.get 1)) (i64.const 1))
  )
  (func (export "bext64_3") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_u (local.get 0) (local.get 1)))
  )
  (func (export "bext64_4") (param i64 i64) (result i64)
    (i64.and (i64.const 1) (i64.shr_s (local.get 0) (local.get 1)))
  )

  (func (export "bexti32_1") (param i32) (result i32)
    (i32.and (i32.shr_u (local.get 0) (i32.const 10)) (i32.const 1))
  )
  (func (export "bexti32_2") (param i32) (result i32)
    (i32.and (i32.shr_s (local.get 0) (i32.const 20)) (i32.const 1))
  )
  (func (export "bexti32_3") (param i32) (result i32)
    (i32.and (i32.shr_u (i32.const 1) (local.get 0) (i32.const 30)))
  )
  (func (export "bexti32_4") (param i32) (result i32)
    (i32.and (i32.shr_s (i32.const 1) (local.get 0) (i32.const 40)))
  )
  (func (export "bexti64_1") (param i64) (result i64)
    (i64.and (i64.shr_u (local.get 0) (i64.const 10)) (i64.const 1))
  )
  (func (export "bexti64_2") (param i64) (result i64)
    (i64.and (i64.shr_s (local.get 0) (i64.const 20)) (i64.const 1))
  )
  (func (export "bexti64_3") (param i64) (result i64)
    (i64.and (i64.shr_u (i64.const 1) (local.get 0) (i64.const 30)))
  )
  (func (export "bexti64_4") (param i64) (result i64)
    (i64.and (i64.shr_s (i64.const 1) (local.get 0) (i64.const 40)))
  )

  (func (export "binv32_1") (param i32 i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "binv32_2") (param i32 i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binv64_1") (param i64 i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "binv64_2") (param i64 i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "binvi32_1") (param i32) (result i32)
    (i32.xor (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "binvi32_2") (param i32) (result i32)
    (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "binvi64_1") (param i64) (result i64)
    (i64.xor (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "binvi64_2") (param i64) (result i64)
    (i64.xor (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )

  (func (export "bset32_1") (param i32 i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (local.get 1)))
  )
  (func (export "bset32_2") (param i32 i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bset64_1") (param i64 i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (local.get 1)))
  )
  (func (export "bset64_2") (param i64 i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (local.get 1)) (local.get 0))
  )
  (func (export "bseti32_1") (param i32) (result i32)
    (i32.or (local.get 0) (i32.shl (i32.const 1) (i32.const 10)))
  )
  (func (export "bseti32_2") (param i32) (result i32)
    (i32.or (i32.shl (i32.const 1) (i32.const 20)) (local.get 0))
  )
  (func (export "bseti64_1") (param i64) (result i64)
    (i64.or (local.get 0) (i64.shl (i64.const 1) (i64.const 30)))
  )
  (func (export "bseti64_2") (param i64) (result i64)
    (i64.or (i64.shl (i64.const 1) (i64.const 40)) (local.get 0))
  )
)
;; function u0:0:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bclr a0,a0,a5
;;   ret
;;
;; function u0:1:
;; block0:
;;   j label1
;; block1:
;;   bclr a0,a0,a1
;;   ret
;;
;; function u0:2:
;; block0:
;;   j label1
;; block1:
;;   bclri a0,a0,4
;;   ret
;;
;; function u0:3:
;; block0:
;;   j label1
;; block1:
;;   bclri a0,a0,20
;;   ret
;;
;; function u0:4:
;; block0:
;;   j label1
;; block1:
;;   bclri a0,a0,4
;;   ret
;;
;; function u0:5:
;; block0:
;;   j label1
;; block1:
;;   bclri a0,a0,52
;;   ret
;;
;; function u0:6:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bext a0,a0,a5
;;   ret
;;
;; function u0:7:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bext a0,a0,a5
;;   ret
;;
;; function u0:8:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bext a0,a0,a5
;;   ret
;;
;; function u0:9:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bext a0,a0,a5
;;   ret
;;
;; function u0:10:
;; block0:
;;   j label1
;; block1:
;;   bext a0,a0,a1
;;   ret
;;
;; function u0:11:
;; block0:
;;   j label1
;; block1:
;;   bext a0,a0,a1
;;   ret
;;
;; function u0:12:
;; block0:
;;   j label1
;; block1:
;;   bext a0,a0,a1
;;   ret
;;
;; function u0:13:
;; block0:
;;   j label1
;; block1:
;;   bext a0,a0,a1
;;   ret
;;
;; function u0:14:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,10
;;   ret
;;
;; function u0:15:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,20
;;   ret
;;
;; function u0:16:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,30
;;   ret
;;
;; function u0:17:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,8
;;   ret
;;
;; function u0:18:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,10
;;   ret
;;
;; function u0:19:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,20
;;   ret
;;
;; function u0:20:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,30
;;   ret
;;
;; function u0:21:
;; block0:
;;   j label1
;; block1:
;;   bexti a0,a0,40
;;   ret
;;
;; function u0:22:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   binv a0,a0,a5
;;   ret
;;
;; function u0:23:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   binv a0,a0,a5
;;   ret
;;
;; function u0:24:
;; block0:
;;   j label1
;; block1:
;;   binv a0,a0,a1
;;   ret
;;
;; function u0:25:
;; block0:
;;   j label1
;; block1:
;;   binv a0,a0,a1
;;   ret
;;
;; function u0:26:
;; block0:
;;   j label1
;; block1:
;;   binvi a0,a0,10
;;   ret
;;
;; function u0:27:
;; block0:
;;   j label1
;; block1:
;;   binvi a0,a0,20
;;   ret
;;
;; function u0:28:
;; block0:
;;   j label1
;; block1:
;;   binvi a0,a0,30
;;   ret
;;
;; function u0:29:
;; block0:
;;   j label1
;; block1:
;;   binvi a0,a0,40
;;   ret
;;
;; function u0:30:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bset a0,a0,a5
;;   ret
;;
;; function u0:31:
;; block0:
;;   j label1
;; block1:
;;   andi a5,a1,31
;;   bset a0,a0,a5
;;   ret
;;
;; function u0:32:
;; block0:
;;   j label1
;; block1:
;;   bset a0,a0,a1
;;   ret
;;
;; function u0:33:
;; block0:
;;   j label1
;; block1:
;;   bset a0,a0,a1
;;   ret
;;
;; function u0:34:
;; block0:
;;   j label1
;; block1:
;;   bseti a0,a0,10
;;   ret
;;
;; function u0:35:
;; block0:
;;   j label1
;; block1:
;;   bseti a0,a0,20
;;   ret
;;
;; function u0:36:
;; block0:
;;   j label1
;; block1:
;;   bseti a0,a0,30
;;   ret
;;
;; function u0:37:
;; block0:
;;   j label1
;; block1:
;;   bseti a0,a0,40
;;   ret
