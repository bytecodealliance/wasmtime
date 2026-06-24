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

;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 104 "VMContext+0x68"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     gv9 = vmctx
;;     gv10 = load.i64 notrap aligned readonly can_move region0 gv9+8
;;     gv11 = load.i64 notrap aligned region1 gv10+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u1:0 sig0
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
;;                                     v20 = iconst.i32 301
;; @00c6                               return v20  ; v20 = 301
;; }
