;;! target = "x86_64"
;;! test = "optimize"

(module
  (type $f (func))
  (func (param $x (ref $f))
    (call_ref $f (local.get $x))
    (call_ref $f (local.get $x))
  )
)
;; function u0:0(i64 vmctx, i64, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 120 ""
;;     region3 = 100 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @001e                               v3 = load.i64 user16 aligned readonly region2 v2+8
;; @001e                               v4 = load.i64 notrap aligned readonly region3 v2+24
;; @001e                               call_indirect sig0, v3(v4, v0)
;; @0022                               call_indirect sig0, v3(v4, v0)
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return
;; }
