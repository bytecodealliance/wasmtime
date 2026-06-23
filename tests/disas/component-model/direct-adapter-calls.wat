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
;; @0080                               v6 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @0080                               v7 = load.i32 notrap aligned region3 v6
;; @0082                               v8 = iconst.i32 1
;; @0084                               v9 = band v7, v8  ; v8 = 1
;; @0075                               v3 = iconst.i32 0
;; @0085                               v11 = icmp eq v9, v3  ; v3 = 0
;; @0086                               brif v11, block7, block8
;;
;;                                 block7:
;; @008a                               v15 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @008a                               v14 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0088                               v13 = iconst.i32 23
;; @008a                               try_call_indirect v15(v14, v0, v13), sig0, block9, [ context v0, default: block6(exn0) ]  ; v13 = 23
;;
;;                                 block9:
;; @008c                               trap user12
;;
;;                                 block8:
;; @008e                               v16 = load.i64 notrap aligned readonly can_move region6 v0+112
;; @008e                               v17 = load.i32 notrap aligned region3 v16
;; @0090                               v18 = iconst.i32 -2
;; @0092                               v19 = band v17, v18  ; v18 = -2
;; @0093                               store notrap aligned region3 v19, v16
;;                                     v45 = iconst.i32 1
;;                                     v46 = bor v17, v45  ; v45 = 1
;; @009c                               store notrap aligned region3 v46, v16
;; @009e                               v26 = load.i64 notrap aligned readonly can_move region7 v0+72
;; @009e                               try_call fn0(v26, v0, v2), sig1, block10(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block10(v27: i32):
;; @00a2                               v29 = load.i32 notrap aligned region3 v6
;;                                     v47 = iconst.i32 -2
;;                                     v48 = band v29, v47  ; v47 = -2
;; @00a7                               store notrap aligned region3 v48, v6
;;                                     v49 = iconst.i32 1
;;                                     v50 = bor v29, v49  ; v49 = 1
;; @00b0                               store notrap aligned region3 v50, v6
;; @00b2                               jump block5
;;
;;                                 block5:
;; @00b3                               jump block2
;;
;;                                 block3:
;;                                     v51 = load.i64 notrap aligned readonly can_move region5 v0+88
;;                                     v52 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @00b6                               v38 = iconst.i32 49
;; @00b8                               call_indirect sig0, v51(v52, v0, v38)  ; v38 = 49
;; @00ba                               trap user12
;;
;;                                 block2:
;; @00bc                               jump block1
;;
;;                                 block1:
;; @00bc                               return v27
;; }
