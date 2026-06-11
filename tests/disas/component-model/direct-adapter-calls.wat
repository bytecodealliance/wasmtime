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
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
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
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move region1 v0+72
;; @00eb                               v3 = iconst.i32 1234
;; @00ee                               v6 = call fn0(v5, v0, v3)  ; v3 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v6
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
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region1 gv3+136
;;     gv5 = load.i64 notrap aligned readonly can_move region5 gv3+112
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0079                               jump block3
;;
;;                                 block5(v6: i64):
;; @0079                               jump block2
;;
;;                                 block3:
;; @007e                               v8 = load.i64 notrap aligned readonly can_move region1 v0+136
;; @007e                               v9 = load.i32 notrap aligned region2 v8
;; @0080                               v10 = iconst.i32 1
;; @0082                               v11 = band v9, v10  ; v10 = 1
;; @0075                               v4 = iconst.i32 0
;; @0083                               v13 = icmp eq v11, v4  ; v4 = 0
;; @0084                               brif v13, block6, block7
;;
;;                                 block6:
;; @0088                               v18 = load.i64 notrap aligned readonly can_move region4 v0+88
;; @0088                               v17 = load.i64 notrap aligned readonly can_move region3 v0+104
;; @0086                               v15 = iconst.i32 23
;; @0088                               try_call_indirect v18(v17, v0, v15), sig0, block8, [ context v0, default: block5(exn0) ]  ; v15 = 23
;;
;;                                 block8:
;; @008a                               trap user12
;;
;;                                 block7:
;; @008c                               v19 = load.i64 notrap aligned readonly can_move region5 v0+112
;; @008c                               v20 = load.i32 notrap aligned region2 v19
;; @008e                               v21 = iconst.i32 -2
;; @0090                               v22 = band v20, v21  ; v21 = -2
;; @0091                               store notrap aligned region2 v22, v19
;;                                     v59 = iconst.i32 1
;;                                     v60 = bor v20, v59  ; v59 = 1
;; @009a                               store notrap aligned region2 v60, v19
;; @009c                               v30 = load.i64 notrap aligned readonly can_move region6 v0+72
;; @009c                               try_call fn0(v30, v0, v2), sig1, block9(ret0), [ context v0, default: block5(exn0) ]
;;
;;                                 block9(v31: i32):
;; @00a0                               v33 = load.i32 notrap aligned region2 v8
;;                                     v61 = iconst.i32 -2
;;                                     v62 = band v33, v61  ; v61 = -2
;; @00a5                               store notrap aligned region2 v62, v8
;;                                     v63 = iconst.i32 1
;;                                     v64 = bor v33, v63  ; v63 = 1
;; @00ae                               store notrap aligned region2 v64, v8
;; @00b0                               jump block4(v31)
;;
;;                                 block4(v5: i32):
;; @00b1                               return v5
;;
;;                                 block2:
;;                                     v65 = load.i64 notrap aligned readonly can_move region4 v0+88
;;                                     v66 = load.i64 notrap aligned readonly can_move region3 v0+104
;; @00b3                               v42 = iconst.i32 49
;; @00b5                               call_indirect sig0, v65(v66, v0, v42)  ; v42 = 49
;; @00b7                               trap user12
;; }
