;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=n"

;; `$C` is instantiated twice from two *different* core instances, but within
;; each instantiation all of its function imports come from one instance. The
;; analysis tracks the *partition* of the imports, not which instance satisfies
;; them, so the two instantiations agree and all three loads still collapse.

(component
  (core module $M
    (func (export "f0"))
    (func (export "f1"))
    (func (export "f2"))
  )
  (core instance $m1 (instantiate $M))
  (core instance $m2 (instantiate $M))

  (core module $C
    (import "" "f0" (func $f0))
    (import "" "f1" (func $f1))
    (import "" "f2" (func $f2))
    (func (export "g")
      call $f0
      call $f1
      call $f2)
  )
  (core instance $c1 (instantiate $C (with "" (instance $m1))))
  (core instance $c2 (instantiate $C (with "" (instance $m2))))
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
;; @0085                               v3 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0085                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0085                               call_indirect sig0, v3(v2, v0)
;; @0087                               v5 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @0087                               call_indirect sig0, v5(v2, v0)
;; @0089                               v7 = load.i64 notrap aligned readonly can_move region3 v0+120
;; @0089                               call_indirect sig0, v7(v2, v0)
;; @008b                               jump block1
;;
;;                                 block1:
;; @008b                               return
;; }
