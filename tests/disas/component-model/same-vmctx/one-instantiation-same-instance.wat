;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=n"

;; `$C` is instantiated once, with all three of its function imports coming from
;; the same core instance `$m`, so all three share a `vmctx`.

(component
  (core module $M
    (func (export "f0"))
    (func (export "f1"))
    (func (export "f2"))
  )
  (core instance $m (instantiate $M))

  (core module $C
    (import "" "f0" (func $f0))
    (import "" "f1" (func $f1))
    (import "" "f2" (func $f2))
    (func (export "g")
      call $f0
      call $f1
      call $f2)
  )
  (core instance $c (instantiate $C (with "" (instance $m))))
)
;; function u1:0(i64 vmctx, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 25 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u0:1 sig0
;;     fn2 = colocated u0:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0082                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0082                               call fn0(v2, v0)
;; @0084                               call fn1(v2, v0)
;; @0086                               call fn2(v2, v0)
;; @0088                               jump block1
;;
;;                                 block1:
;; @0088                               return
;; }
