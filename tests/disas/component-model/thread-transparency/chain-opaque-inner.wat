;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Opacity does not propagate outwards along a call chain: `canon
;; context.{get,set}` in $Inner makes the $Mid -> $Inner adapter opaque, but the
;; $Outer -> $Mid adapter above it stays transparent. Both adapters are inlined
;; into the function below, so we only have one `explicit_slot 24` for the
;; `VMDeferredThread`, not multiple slots.

(component
  (component $Inner
    (core func $cget (canon context.get i32 0))
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "cget" (func $cget (result i32)))
      (import "" "cset" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        (call $cset (i32.const 0x5555))
        (i32.add (local.get 0) (call $cget))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance
          (export "cget" (func $cget))
          (export "cset" (func $cset))
        ))
      )
    )
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
;;     ss0 = explicit_slot 24, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197584 "VMComponentContext+0x50"
;;     region5 = 738197568 "VMComponentContext+0x40"
;;     region6 = 67109000 "VMStoreContext+0x88"
;;     region7 = 1006632960 "VMDeferredThread+0x0"
;;     region8 = 1006632968 "VMDeferredThread+0x8"
;;     region9 = 1006632972 "VMDeferredThread+0xc"
;;     region10 = 67108992 "VMStoreContext+0x80"
;;     region11 = 1006632976 "VMDeferredThread+0x10"
;;     region12 = 67108996 "VMStoreContext+0x84"
;;     region13 = 1006632980 "VMDeferredThread+0x14"
;;     region14 = 738197552 "VMComponentContext+0x30"
;;     region15 = 1207959560 "VMFunctionImport+0x8"
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
;;     sig5 = (i64 vmctx, i64, i32, i32) tail
;;     sig6 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig7 = (i64 vmctx, i64, i32) tail
;;     sig8 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u4:0 sig0
;;     fn1 = colocated u1:0 sig2
;;     fn2 = colocated u3:0 sig3
;;     fn3 = colocated u0:0 sig6
;;     fn4 = colocated u2147483648:18 sig7
;;     fn5 = colocated u2147483648:17 sig8
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0225                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @0225                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
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
;;                                     v22 = load.i64 notrap aligned readonly can_move region3 v20+232
;;                                     v23 = load.i32 notrap aligned region5 v22
;;                                     trapz v23, user26
;;                                     jump block21
;;
;;                                 block21:
;;                                     v28 = load.i64 notrap aligned readonly can_move region0 v20+8
;;                                     v29 = load.i64 notrap aligned region6 v28+136
;;                                     v27 = stack_addr.i64 ss0
;;                                     store notrap aligned region7 v29, v27
;;                                     v11 = iconst.i32 0
;;                                     store notrap aligned region8 v11, v27+8  ; v11 = 0
;;                                     v25 = iconst.i32 1
;;                                     store notrap aligned region9 v25, v27+12  ; v25 = 1
;;                                     v30 = load.i32 notrap aligned region10 v28+128
;;                                     store notrap aligned region11 v30, v27+16
;;                                     store notrap aligned region10 v11, v28+128  ; v11 = 0
;;                                     v32 = load.i32 notrap aligned region12 v28+132
;;                                     store notrap aligned region13 v32, v27+20
;;                                     store notrap aligned region12 v11, v28+132  ; v11 = 0
;;                                     store notrap aligned region6 v27, v28+136
;;                                     v34 = load.i64 notrap aligned readonly can_move region3 v20+208
;;                                     v35 = load.i32 notrap aligned region14 v34
;;                                     jump block28
;;
;;                                 block28:
;;                                     v40 = iconst.i32 0x5555
;;                                     v39 = load.i64 notrap aligned readonly can_move region2 v20+72
;;                                     v42 = load.i64 notrap aligned readonly can_move region0 v39+8
;;                                     store notrap aligned region10 v40, v42+128  ; v40 = 0x5555
;;                                     jump block29
;;
;;                                 block29:
;;                                     jump block23
;;
;;                                 block23:
;;                                     jump block24
;;
;;                                 block24:
;;                                     store.i64 notrap aligned region6 v29, v28+136
;;                                     store.i32 notrap aligned region10 v30, v28+128
;;                                     store.i32 notrap aligned region12 v32, v28+132
;;                                     jump block26
;;
;;                                 block26:
;;                                     store.i32 notrap aligned region5 v23, v22
;;                                     jump block19
;;
;;                                 block19:
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block15
;;
;;                                 block15:
;;                                     jump block30
;;
;;                                 block30:
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block11
;;
;;                                 block11:
;;                                     store.i32 notrap aligned region4 v13, v12
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block31
;;
;;                                 block31:
;; @0227                               jump block1
;;
;;                                 block1:
;;                                     v77 = iconst.i32 0x5a19
;; @0227                               return v77  ; v77 = 0x5a19
;; }
