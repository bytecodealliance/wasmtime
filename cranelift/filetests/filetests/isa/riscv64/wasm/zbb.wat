;;! target = "riscv64"
;;! compile = true
;;! settings = ["has_zbb", "opt_level=speed"]

(module
  (func (export "rolw") (param i32 i32) (result i32)
    (i32.rotl (local.get 0) (local.get 1)))
  (func (export "rol") (param i64 i64) (result i64)
    (i64.rotl (local.get 0) (local.get 1)))
  (func (export "rolwi") (param i32 ) (result i32)
    (i32.rotl (local.get 0) (i32.const 100)))
  (func (export "roli") (param i64) (result i64)
    (i64.rotl (local.get 0) (i64.const 40)))

  (func (export "rorw") (param i32 i32) (result i32)
    (i32.rotr (local.get 0) (local.get 1)))
  (func (export "ror") (param i64 i64) (result i64)
    (i64.rotr (local.get 0) (local.get 1)))
  (func (export "rorwi") (param i32 ) (result i32)
    (i32.rotr (local.get 0) (i32.const 100)))
  (func (export "rori") (param i64) (result i64)
    (i64.rotr (local.get 0) (i64.const 40)))

  (func (export "xnor32_1") (param i32 i32) (result i32)
    (i32.xor (i32.xor (local.get 0) (local.get 1)) (i32.const -1)))
  (func (export "xnor32_2") (param i32 i32) (result i32)
    (i32.xor (i32.const -1) (i32.xor (local.get 0) (local.get 1))))
  (func (export "xnor64_1") (param i64 i64) (result i64)
    (i64.xor (i64.xor (local.get 0) (local.get 1)) (i64.const -1)))
  (func (export "xnor64_2") (param i64 i64) (result i64)
    (i64.xor (i64.const -1) (i64.xor (local.get 0) (local.get 1))))
)
;; function u0:0:
;; block0:
;;   j label1
;; block1:
;;   rolw a0,a0,a1
;;   ret
;;
;; function u0:1:
;; block0:
;;   j label1
;; block1:
;;   rol a0,a0,a1
;;   ret
;;
;; function u0:2:
;; block0:
;;   j label1
;; block1:
;;   roriw a0,a0,28
;;   ret
;;
;; function u0:3:
;; block0:
;;   j label1
;; block1:
;;   rori a0,a0,24
;;   ret
;;
;; function u0:4:
;; block0:
;;   j label1
;; block1:
;;   rorw a0,a0,a1
;;   ret
;;
;; function u0:5:
;; block0:
;;   j label1
;; block1:
;;   ror a0,a0,a1
;;   ret
;;
;; function u0:6:
;; block0:
;;   j label1
;; block1:
;;   roriw a0,a0,4
;;   ret
;;
;; function u0:7:
;; block0:
;;   j label1
;; block1:
;;   rori a0,a0,40
;;   ret
;;
;; function u0:8:
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a0,a1
;;   ret
;;
;; function u0:9:
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a0,a1
;;   ret
;;
;; function u0:10:
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a0,a1
;;   ret
;;
;; function u0:11:
;; block0:
;;   j label1
;; block1:
;;   xnor a0,a0,a1
;;   ret
