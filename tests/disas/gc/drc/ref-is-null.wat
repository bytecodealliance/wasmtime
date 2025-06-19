;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.is_null (local.get 0))
  )
  (func (param (ref any)) (result i32)
    (ref.is_null (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0023                               jump block1
;;
;;                                 block1:
;;                                     v6 = iconst.i32 0
;;                                     v4 = icmp.i32 eq v2, v6  ; v6 = 0
;;                                     v5 = uextend.i32 v4
;; @0023                               return v5
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0029                               jump block1
;;
;;                                 block1:
;;                                     v4 = iconst.i32 0
;; @0029                               return v4  ; v4 = 0
;; }
