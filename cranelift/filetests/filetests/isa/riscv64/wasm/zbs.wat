;;! target = "riscv64"
;;! compile = true
;;! settings = ["has_zbs", "opt_level=speed"]

(module
  (func (export "bclr32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (local.get 1)) (i32.const -1)))
  )
  (func (export "bclr64") (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (local.get 1)) (i64.const -1)))
  )
  (func (export "bclri32_4") (param i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (i32.const 4)) (i32.const -1)))
  )
  (func (export "bclri32_20") (param i32) (result i32)
    (i32.and (local.get 0) (i32.xor (i32.shl (i32.const 1) (i32.const 20)) (i32.const -1)))
  )
  (func (export "bclri64_4") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 4)) (i64.const -1)))
  )
  (func (export "bclri64_52") (param i64) (result i64)
    (i64.and (local.get 0) (i64.xor (i64.shl (i64.const 1) (i64.const 52)) (i64.const -1)))
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
