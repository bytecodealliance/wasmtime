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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
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
;;     region1 = 72 "VMContext+0x48"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v4 = load.i64 notrap aligned readonly can_move region1 v0+72
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
;;     region1 = 136 "VMContext+0x88"
;;     region2 = 1610612736 "PublicGlobal"
;;     region3 = 104 "VMContext+0x68"
;;     region4 = 88 "VMContext+0x58"
;;     region5 = 112 "VMContext+0x70"
;;     region6 = 72 "VMContext+0x48"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @007b                               jump block4
;;
;;                                 block6(v7: i64):
;; @007b                               jump block3
;;
;;                                 block4:
;; @0080                               v9 = load.i64 notrap aligned readonly can_move region1 v0+136
;; @0080                               v10 = load.i32 notrap aligned region2 v9
;; @0082                               v11 = iconst.i32 1
;; @0084                               v12 = band v10, v11  ; v11 = 1
;; @0075                               v4 = iconst.i32 0
;; @0085                               v14 = icmp eq v12, v4  ; v4 = 0
;; @0086                               brif v14, block7, block8
;;
;;                                 block7:
;; @008a                               v18 = load.i64 notrap aligned readonly can_move region4 v0+88
;; @008a                               v17 = load.i64 notrap aligned readonly can_move region3 v0+104
;; @0088                               v16 = iconst.i32 23
;; @008a                               try_call_indirect v18(v17, v0, v16), sig0, block9, [ context v0, default: block6(exn0) ]  ; v16 = 23
;;
;;                                 block9:
;; @008c                               trap user12
;;
;;                                 block8:
;; @008e                               v19 = load.i64 notrap aligned readonly can_move region5 v0+112
;; @008e                               v20 = load.i32 notrap aligned region2 v19
;; @0090                               v21 = iconst.i32 -2
;; @0092                               v22 = band v20, v21  ; v21 = -2
;; @0093                               store notrap aligned region2 v22, v19
;;                                     v48 = iconst.i32 1
;;                                     v49 = bor v20, v48  ; v48 = 1
;; @009c                               store notrap aligned region2 v49, v19
;; @009e                               v29 = load.i64 notrap aligned readonly can_move region6 v0+72
;; @009e                               try_call fn0(v29, v0, v2), sig1, block10(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block10(v30: i32):
;; @00a2                               v32 = load.i32 notrap aligned region2 v9
;;                                     v50 = iconst.i32 -2
;;                                     v51 = band v32, v50  ; v50 = -2
;; @00a7                               store notrap aligned region2 v51, v9
;;                                     v52 = iconst.i32 1
;;                                     v53 = bor v32, v52  ; v52 = 1
;; @00b0                               store notrap aligned region2 v53, v9
;; @00b2                               jump block5(v30)
;;
;;                                 block5(v6: i32):
;; @00b3                               jump block2(v6)
;;
;;                                 block3:
;;                                     v54 = load.i64 notrap aligned readonly can_move region4 v0+88
;;                                     v55 = load.i64 notrap aligned readonly can_move region3 v0+104
;; @00b6                               v41 = iconst.i32 49
;; @00b8                               call_indirect sig0, v54(v55, v0, v41)  ; v41 = 49
;; @00ba                               trap user12
;;
;;                                 block2(v5: i32):
;; @00bc                               jump block1(v5)
;;
;;                                 block1(v3: i32):
;; @00bc                               return v3
;; }
