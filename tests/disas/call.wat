;;! target = "x86_64"

(module
  (func $main (local i32)
    (local.set 0 (i32.const 0))
    (drop (call $inc))
  )
  (func $inc (result i32)
    (i32.const 1)
  )
  (start $main)
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u0:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @001f                               v2 = iconst.i32 0
;; @0021                               v3 = iconst.i32 0
;; @0025                               v4 = call fn0(v0, v0)
;; @0028                               jump block1
;;
;;                                 block1:
;; @0028                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002b                               v3 = iconst.i32 1
;; @002d                               jump block1(v3)  ; v3 = 1
;;
;;                                 block1(v2: i32):
;; @002d                               return v2
;; }
