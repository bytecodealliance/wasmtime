;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref any) (local.get 0))
  )
  (func (param (ref any)) (result i32)
    (ref.test (ref any) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0025                               jump block1
;;
;;                                 block1:
;; @0022                               v7 = iconst.i32 1
;;                                     v9 = iconst.i32 0
;;                                     v14 = select v2, v7, v9  ; v7 = 1, v9 = 0
;; @0025                               return v14
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               jump block1
;;
;;                                 block1:
;; @002a                               v7 = iconst.i32 1
;;                                     v9 = iconst.i32 0
;;                                     v14 = select v2, v7, v9  ; v7 = 1, v9 = 0
;; @002d                               return v14
;; }
