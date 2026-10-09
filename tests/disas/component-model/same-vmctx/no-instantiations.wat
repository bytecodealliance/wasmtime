;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-C inlining=n"

;; A module that is never instantiated has nothing to contradict the analysis's
;; initial state, where all of its function imports trivially share one `vmctx`,
;; so all three calls share one callee `vmctx`.

(component
  (core module $C
    (import "" "f0" (func $f0))
    (import "" "f1" (func $f1))
    (import "" "f2" (func $f2))
    (func (export "g")
      call $f0
      call $f1
      call $f2)
  )
)
;; function u0:0(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 25 ""
;;     region3 = 68 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003d                               v3 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @003d                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @003d                               call_indirect sig0, v3(v2, v0)
;; @003f                               v5 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @003f                               call_indirect sig0, v5(v2, v0)
;; @0041                               v7 = load.i64 notrap aligned readonly can_move region3 v0+120
;; @0041                               call_indirect sig0, v7(v2, v0)
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
