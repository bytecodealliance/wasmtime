;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=n"

;; `$C` is instantiated once, with its first two function imports coming from
;; `$a` and its last two from `$b`. Each half shares a `vmctx` internally but
;; not with the other half, leaving exactly two `vmctx` loads, one per half.

(component
  (core module $M
    (func (export "f0"))
    (func (export "f1"))
    (func (export "f2"))
    (func (export "f3"))
  )
  (core instance $a (instantiate $M))
  (core instance $b (instantiate $M))

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
  (core instance $c (instantiate $C (with "" (instance
    (export "f0" (func $a "f0"))
    (export "f1" (func $a "f1"))
    (export "f2" (func $b "f2"))
    (export "f3" (func $b "f3"))
  ))))
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
;;     fn3 = colocated u0:3 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0094                               v2 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0094                               call fn0(v2, v0)
;; @0096                               call fn1(v2, v0)
;; @0098                               v4 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @0098                               call fn2(v4, v0)
;; @009a                               call fn3(v4, v0)
;; @009c                               jump block1
;;
;;                                 block1:
;; @009c                               return
;; }
