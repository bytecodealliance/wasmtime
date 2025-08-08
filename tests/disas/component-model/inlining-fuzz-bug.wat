;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-C inlining=y"

(component
  (core module $A
    (func (export "f0") (result i32)
      (i32.const 100)
    )
    (func (export "f1") (result i32)
      (i32.const 101)
    )
  )

  (core module $B
    (import "a" "f0" (func $f0 (result i32)))
    (import "a" "f1" (func $f1 (result i32)))
    (func (export "f2") (result i32)
      (i32.add (call $f0) (call $f1))
    )
  )

  (core module $C
    (import "b" "f2" (func $f2 (result i32)))
    (func (export "f3") (result i32)
      (i32.add (i32.const 100) (call $f2))
    )
  )

  (core instance $a (instantiate $A))
  (core instance $b (instantiate $B (with "a" (instance $a))))
  (core instance $c (instantiate $C (with "b" (instance $b))))

  (func (export "f") (result u32)
    (canon lift (core func $c "f3"))
  )
)

;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = vmctx
;;     gv5 = load.i64 notrap aligned readonly gv4+8
;;     gv6 = load.i64 notrap aligned gv5+16
;;     gv7 = vmctx
;;     gv8 = vmctx
;;     gv9 = load.i64 notrap aligned readonly gv8+8
;;     gv10 = load.i64 notrap aligned gv9+16
;;     gv11 = vmctx
;;     gv12 = load.i64 notrap aligned readonly gv11+8
;;     gv13 = load.i64 notrap aligned gv12+16
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u0:0 sig1
;;     fn2 = colocated u0:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00c3                               jump block2
;;
;;                                 block2:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block5
;;
;;                                 block5:
;;                                     jump block6
;;
;;                                 block6:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block9
;;
;;                                 block9:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block10
;;
;;                                 block10:
;; @00c6                               jump block1
;;
;;                                 block1:
;;                                     v26 = iconst.i32 301
;; @00c6                               return v26  ; v26 = 301
;; }
