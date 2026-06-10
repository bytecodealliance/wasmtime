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
;; @0077                               v5 = load.i64 notrap aligned readonly can_move region1 v0+136
;; @0077                               v6 = load.i32 notrap aligned region2 v5
;; @0079                               v7 = iconst.i32 1
;; @007b                               v8 = band v6, v7  ; v7 = 1
;; @0075                               v4 = iconst.i32 0
;; @007c                               v10 = icmp eq v8, v4  ; v4 = 0
;; @007d                               brif v10, block2, block3
;;
;;                                 block2:
;; @0081                               v15 = load.i64 notrap aligned readonly can_move region4 v0+88
;; @0081                               v14 = load.i64 notrap aligned readonly can_move region3 v0+104
;; @007f                               v12 = iconst.i32 23
;; @0081                               call_indirect sig0, v15(v14, v0, v12)  ; v12 = 23
;; @0083                               trap user12
;;
;;                                 block3:
;; @0085                               v16 = load.i64 notrap aligned readonly can_move region5 v0+112
;; @0085                               v17 = load.i32 notrap aligned region2 v16
;; @0087                               v18 = iconst.i32 -2
;; @0089                               v19 = band v17, v18  ; v18 = -2
;; @008a                               store notrap aligned region2 v19, v16
;;                                     v52 = iconst.i32 1
;;                                     v53 = bor v17, v52  ; v52 = 1
;; @0093                               store notrap aligned region2 v53, v16
;; @0095                               v27 = load.i64 notrap aligned readonly can_move region6 v0+72
;; @0095                               v28 = call fn0(v27, v0, v2)
;; @0099                               v30 = load.i32 notrap aligned region2 v5
;; @009d                               v32 = band v30, v18  ; v18 = -2
;; @009e                               store notrap aligned region2 v32, v5
;;                                     v54 = bor v30, v52  ; v52 = 1
;; @00a7                               store notrap aligned region2 v54, v5
;; @00a9                               jump block1
;;
;;                                 block1:
;; @00a9                               return v28
;; }
