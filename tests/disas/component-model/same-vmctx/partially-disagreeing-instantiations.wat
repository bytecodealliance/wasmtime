;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=n"

;; `$C` is instantiated twice: once with all of its function imports coming from
;; a single instance, and once with the first half coming from `$m1` and the
;; second half from `$m2`, so we get two `vmctx` loads.

(component
  (core module $M
    (func (export "f0"))
    (func (export "f1"))
    (func (export "f2"))
    (func (export "f3"))
  )
  (core instance $m0 (instantiate $M))
  (core instance $m1 (instantiate $M))
  (core instance $m2 (instantiate $M))

  (core module $C
    (import "" "f0" (func $f0))
    (import "" "f1" (func $f1))
    (import "" "f2" (func $f2))
    (import "" "f3" (func $f3))
    (func (export "g")
      call $f0
      call $f1
      call $f2
      call $f3)
  )
  (core instance $c1 (instantiate $C (with "" (instance $m0))))
  (core instance $c2 (instantiate $C (with "" (instance
    (export "f0" (func $m1 "f0"))
    (export "f1" (func $m1 "f1"))
    (export "f2" (func $m2 "f2"))
    (export "f3" (func $m2 "f3"))
  ))))
)
;; function u1:0(i64 vmctx, i64) tail {
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
;; @0097                               v3 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0097                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0097                               call_indirect sig0, v3(v2, v0)
;; @0099                               v5 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @0099                               call_indirect sig0, v5(v2, v0)
;; @009b                               v7 = load.i64 notrap aligned readonly can_move region3 v0+120
;; @009b                               v6 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @009b                               call_indirect sig0, v7(v6, v0)
;; @009d                               v9 = load.i64 notrap aligned readonly can_move region3 v0+152
;; @009d                               call_indirect sig0, v9(v6, v0)
;; @009f                               jump block1
;;
;;                                 block1:
;; @009f                               return
;; }
