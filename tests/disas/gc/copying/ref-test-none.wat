;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (func (param anyref) (result i32)
    (ref.test (ref none) (local.get 0))
  )
  (func (param anyref) (result i32)
    (ref.test (ref null none) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               jump block1
;;
;;                                 block1:
;; @001c                               v4 = iconst.i32 0
;; @001f                               return v4  ; v4 = 0
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0027                               jump block1
;;
;;                                 block1:
;; @0024                               v4 = iconst.i32 0
;; @0024                               v5 = icmp.i32 eq v2, v4  ; v4 = 0
;; @0024                               v6 = uextend.i32 v5
;; @0027                               return v6
;; }
