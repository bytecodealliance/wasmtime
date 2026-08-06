;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Transparency composes along a call chain: with $Outer -> $Mid -> $Inner all
;; clean, both adapters are thread-transparent and the fully-inlined function
;; below has no `enter-sync-call`/`exit-sync-call` calls.

(component
  (component $Inner
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 2))
      )
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $Mid
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "f'") (param i32) (result i32)
        (i32.add (call $f' (local.get 0)) (i32.const 20))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance (export "f'" (func $f'))))
      )
    )
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $Outer
    (import "f" (func $f (param "x" u32) (result u32)))
    (core func $f' (canon lower (func $f)))
    (core module $M
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1200))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance (export "f'" (func $f'))))
      )
    )
    (func (export "g") (result u32)
      (canon lift (core func $m "g'"))
    )
  )

  (instance $inner (instantiate $Inner))
  (instance $mid (instantiate $Mid (with "f" (func $inner "f"))))
  (instance $outer (instantiate $Outer (with "f" (func $mid "f"))))

  (export "g" (func $outer "g"))
)
;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197584 "VMComponentContext+0x50"
;;     region5 = 738197568 "VMComponentContext+0x40"
;;     region6 = 738197552 "VMComponentContext+0x30"
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
;;     gv12 = vmctx
;;     gv13 = load.i64 notrap aligned readonly can_move region0 gv12+8
;;     gv14 = load.i64 notrap aligned region1 gv13+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig3 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig4 = (i64 vmctx, i64) tail
;;     sig5 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u4:0 sig0
;;     fn1 = colocated u1:0 sig2
;;     fn2 = colocated u3:0 sig3
;;     fn3 = colocated u0:0 sig5
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @01c8                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @01c8                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v12 = load.i64 notrap aligned readonly can_move region3 v3+168
;;                                     v13 = load.i32 notrap aligned region4 v12
;;                                     trapz v13, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v14 = load.i64 notrap aligned readonly can_move region3 v3+144
;;                                     v15 = load.i32 notrap aligned region5 v14
;;                                     jump block12
;;
;;                                 block12:
;;                                     jump block14
;;
;;                                 block14:
;;                                     jump block18
;;
;;                                 block18:
;;                                     v19 = load.i64 notrap aligned readonly can_move region2 v3+72
;;                                     v20 = load.i64 notrap aligned readonly can_move region2 v19+72
;;                                     v22 = load.i64 notrap aligned readonly can_move region3 v20+168
;;                                     v23 = load.i32 notrap aligned region5 v22
;;                                     trapz v23, user26
;;                                     jump block21
;;
;;                                 block21:
;;                                     v24 = load.i64 notrap aligned readonly can_move region3 v20+144
;;                                     v25 = load.i32 notrap aligned region6 v24
;;                                     jump block24
;;
;;                                 block24:
;;                                     jump block25
;;
;;                                 block25:
;;                                     jump block23
;;
;;                                 block23:
;;                                     jump block19
;;
;;                                 block19:
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block15
;;
;;                                 block15:
;;                                     jump block26
;;
;;                                 block26:
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block27
;;
;;                                 block27:
;; @01ca                               jump block1
;;
;;                                 block1:
;;                                     v48 = iconst.i32 1222
;; @01ca                               return v48  ; v48 = 1222
;; }
