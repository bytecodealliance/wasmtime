;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; The following component links two sub-components together and each are only
;; instantiated the once, so we statically know what their core modules'
;; function imports will be, and can emit direct calls to those function imports
;; instead of indirect calls through the imports table. There should be zero
;; `call_indirect`s in the disassembly.

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003b                               jump block1
;;
;;                                 block1:
;; @0038                               v3 = iconst.i32 42
;;                                     v4 = iadd.i32 v2, v3  ; v3 = 42
;; @003b                               return v4
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @00eb                               v2 = iconst.i32 1234
;; @00ee                               v4 = call fn0(v3, v0, v2)  ; v2 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v4
;; }
;;
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 136 "VMContext+0x88"
;;     region3 = 1610612736 "PublicGlobal"
;;     region4 = 104 "VMContext+0x68"
;;     region5 = 88 "VMContext+0x58"
;;     region6 = 112 "VMContext+0x70"
;;     region7 = 72 "VMContext+0x48"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @007b                               jump block4
;;
;;                                 block6(v4: i64):
;; @007b                               jump block3
;;
;;                                 block4:
;; @0082                               v6 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @0082                               v7 = load.i32 notrap aligned region3 v6
;; @0086                               brif v7, block7, block8
;;
;;                                 block8:
;; @008a                               v10 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @008a                               v9 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0088                               v8 = iconst.i32 23
;; @008a                               try_call_indirect v10(v9, v0, v8), sig0, block9, [ context v0, default: block6(exn0) ]  ; v8 = 23
;;
;;                                 block9:
;; @008c                               trap user12
;;
;;                                 block7:
;; @008e                               v11 = load.i64 notrap aligned readonly can_move region6 v0+112
;; @008e                               v12 = load.i32 notrap aligned region3 v11
;; @0075                               v3 = iconst.i32 0
;; @0094                               store notrap aligned region3 v3, v11  ; v3 = 0
;; @009a                               store notrap aligned region3 v12, v11
;; @009c                               v16 = load.i64 notrap aligned readonly can_move region7 v0+72
;; @009c                               try_call fn0(v16, v0, v2), sig1, block10(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block10(v17: i32):
;;                                     v24 = iconst.i32 0
;; @00a2                               store notrap aligned region3 v24, v6  ; v24 = 0
;; @00a8                               store.i32 notrap aligned region3 v7, v6
;; @00aa                               jump block5
;;
;;                                 block5:
;; @00ab                               jump block2
;;
;;                                 block3:
;;                                     v25 = load.i64 notrap aligned readonly can_move region5 v0+88
;;                                     v26 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @00ae                               v21 = iconst.i32 49
;; @00b0                               call_indirect sig0, v25(v26, v0, v21)  ; v21 = 49
;; @00b2                               trap user12
;;
;;                                 block2:
;; @00b4                               jump block1
;;
;;                                 block1:
;; @00b4                               return v17
;; }
