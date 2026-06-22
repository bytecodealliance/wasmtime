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
;; @0038                               v4 = iconst.i32 42
;;                                     v5 = iadd.i32 v2, v4  ; v4 = 42
;; @003b                               return v5
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
;; @00ee                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @00eb                               v3 = iconst.i32 1234
;; @00ee                               v5 = call fn0(v4, v0, v3)  ; v3 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v5
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
;;                                 block6(v5: i64):
;; @007b                               jump block3
;;
;;                                 block4:
;; @0080                               v7 = load.i64 notrap aligned readonly can_move region2 v0+136
;; @0080                               v8 = load.i32 notrap aligned region3 v7
;; @0082                               v9 = iconst.i32 1
;; @0084                               v10 = band v8, v9  ; v9 = 1
;; @0075                               v4 = iconst.i32 0
;; @0085                               v12 = icmp eq v10, v4  ; v4 = 0
;; @0086                               brif v12, block7, block8
;;
;;                                 block7:
;; @008a                               v16 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @008a                               v15 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0088                               v14 = iconst.i32 23
;; @008a                               try_call_indirect v16(v15, v0, v14), sig0, block9, [ context v0, default: block6(exn0) ]  ; v14 = 23
;;
;;                                 block9:
;; @008c                               trap user12
;;
;;                                 block8:
;; @008e                               v17 = load.i64 notrap aligned readonly can_move region6 v0+112
;; @008e                               v18 = load.i32 notrap aligned region3 v17
;; @0090                               v19 = iconst.i32 -2
;; @0092                               v20 = band v18, v19  ; v19 = -2
;; @0093                               store notrap aligned region3 v20, v17
;;                                     v46 = iconst.i32 1
;;                                     v47 = bor v18, v46  ; v46 = 1
;; @009c                               store notrap aligned region3 v47, v17
;; @009e                               v27 = load.i64 notrap aligned readonly can_move region7 v0+72
;; @009e                               try_call fn0(v27, v0, v2), sig1, block10(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block10(v28: i32):
;; @00a2                               v30 = load.i32 notrap aligned region3 v7
;;                                     v48 = iconst.i32 -2
;;                                     v49 = band v30, v48  ; v48 = -2
;; @00a7                               store notrap aligned region3 v49, v7
;;                                     v50 = iconst.i32 1
;;                                     v51 = bor v30, v50  ; v50 = 1
;; @00b0                               store notrap aligned region3 v51, v7
;; @00b2                               jump block5
;;
;;                                 block5:
;; @00b3                               jump block2
;;
;;                                 block3:
;;                                     v52 = load.i64 notrap aligned readonly can_move region5 v0+88
;;                                     v53 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @00b6                               v39 = iconst.i32 49
;; @00b8                               call_indirect sig0, v52(v53, v0, v39)  ; v39 = 49
;; @00ba                               trap user12
;;
;;                                 block2:
;; @00bc                               jump block1(v28)
;;
;;                                 block1(v3: i32):
;; @00bc                               return v3
;; }
