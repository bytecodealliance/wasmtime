;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; Opacity is per-adapter, not per-caller: $Caller calls both a clean component
;; instance and one declaring `canon context.{get,set}` from the same core
;; function, and only the latter adapter keeps its window, so exactly one
;; `explicit_slot 24` (the frame-local `VMDeferredThread`) appears below.

(component
  (component $Clean
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

  (component $Dirty
    (core func $cset (canon context.set i32 0))
    (core module $M
      (import "" "cset" (func $cset (param i32)))
      (func (export "f'") (param i32) (result i32)
        (call $cset (i32.const 0x5555))
        (i32.add (local.get 0) (i32.const 20))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance (export "cset" (func $cset))))
      )
    )
    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $Caller
    (import "clean" (func $clean (param "x" u32) (result u32)))
    (import "dirty" (func $dirty (param "x" u32) (result u32)))
    (core func $clean' (canon lower (func $clean)))
    (core func $dirty' (canon lower (func $dirty)))
    (core module $M
      (import "" "clean" (func $clean (param i32) (result i32)))
      (import "" "dirty" (func $dirty (param i32) (result i32)))
      (func (export "g'") (result i32)
        (i32.add (call $clean (i32.const 1200)) (call $dirty (i32.const 0)))
      )
    )
    (core instance $m
      (instantiate $M
        (with "" (instance
          (export "clean" (func $clean'))
          (export "dirty" (func $dirty'))
        ))
      )
    )
    (func (export "g") (result u32)
      (canon lift (core func $m "g'"))
    )
  )

  (instance $clean (instantiate $Clean))
  (instance $dirty (instantiate $Dirty))
  (instance $caller
    (instantiate $Caller
      (with "clean" (func $clean "f"))
      (with "dirty" (func $dirty "f"))
    )
  )

  (export "g" (func $caller "g"))
)
;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 24, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197584 "VMComponentContext+0x50"
;;     region5 = 738197552 "VMComponentContext+0x30"
;;     region6 = 67109000 "VMStoreContext+0x88"
;;     region7 = 1006632960 "VMDeferredThread+0x0"
;;     region8 = 1006632968 "VMDeferredThread+0x8"
;;     region9 = 1006632972 "VMDeferredThread+0xc"
;;     region10 = 67108992 "VMStoreContext+0x80"
;;     region11 = 1006632976 "VMDeferredThread+0x10"
;;     region12 = 67108996 "VMStoreContext+0x84"
;;     region13 = 1006632980 "VMDeferredThread+0x14"
;;     region14 = 738197568 "VMComponentContext+0x40"
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
;;     sig3 = (i64 vmctx, i64) tail
;;     sig4 = (i64 vmctx, i64, i32, i32) tail
;;     sig5 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig6 = (i64 vmctx, i64, i32) tail
;;     fn0 = colocated u3:0 sig0
;;     fn1 = colocated u3:1 sig0
;;     fn2 = colocated u0:0 sig2
;;     fn3 = colocated u1:0 sig5
;;     fn4 = colocated u2147483648:18 sig6
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @01ea                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @01ea                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v13 = load.i64 notrap aligned readonly can_move region3 v3+264
;;                                     v14 = load.i32 notrap aligned region4 v13
;;                                     trapz v14, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v15 = load.i64 notrap aligned readonly can_move region3 v3+240
;;                                     v16 = load.i32 notrap aligned region5 v15
;;                                     jump block12
;;
;;                                 block12:
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
;;                                     jump block14
;;
;;                                 block14:
;; @01ee                               jump block15
;;
;;                                 block15:
;;                                     jump block19
;;
;;                                 block19:
;; @01ee                               v6 = load.i64 notrap aligned readonly can_move region2 v0+104
;;                                     v31 = load.i64 notrap aligned readonly can_move region3 v6+264
;;                                     v32 = load.i32 notrap aligned region4 v31
;;                                     trapz v32, user26
;;                                     jump block22
;;
;;                                 block22:
;;                                     v37 = load.i64 notrap aligned readonly can_move region0 v6+8
;;                                     v38 = load.i64 notrap aligned region6 v37+136
;;                                     v36 = stack_addr.i64 ss0
;;                                     store notrap aligned region7 v38, v36
;;                                     v12 = iconst.i32 0
;;                                     store notrap aligned region8 v12, v36+8  ; v12 = 0
;;                                     v21 = iconst.i32 2
;;                                     store notrap aligned region9 v21, v36+12  ; v21 = 2
;;                                     v39 = load.i32 notrap aligned region10 v37+128
;;                                     store notrap aligned region11 v39, v36+16
;;                                     store notrap aligned region10 v12, v37+128  ; v12 = 0
;;                                     v41 = load.i32 notrap aligned region12 v37+132
;;                                     store notrap aligned region13 v41, v36+20
;;                                     store notrap aligned region12 v12, v37+132  ; v12 = 0
;;                                     store notrap aligned region6 v36, v37+136
;;                                     v43 = load.i64 notrap aligned readonly can_move region3 v6+288
;;                                     v44 = load.i32 notrap aligned region14 v43
;;                                     jump block29
;;
;;                                 block29:
;;                                     v49 = iconst.i32 0x5555
;;                                     v48 = load.i64 notrap aligned readonly can_move region2 v6+168
;;                                     v51 = load.i64 notrap aligned readonly can_move region0 v48+8
;;                                     store notrap aligned region10 v49, v51+128  ; v49 = 0x5555
;;                                     jump block30
;;
;;                                 block30:
;;                                     jump block24
;;
;;                                 block24:
;;                                     jump block25
;;
;;                                 block25:
;;                                     store.i64 notrap aligned region6 v38, v37+136
;;                                     store.i32 notrap aligned region10 v39, v37+128
;;                                     store.i32 notrap aligned region12 v41, v37+132
;;                                     jump block27
;;
;;                                 block27:
;;                                     store.i32 notrap aligned region4 v32, v31
;;                                     jump block20
;;
;;                                 block20:
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block31
;;
;;                                 block31:
;; @01f1                               jump block1
;;
;;                                 block1:
;;                                     v81 = iconst.i32 1222
;; @01f1                               return v81  ; v81 = 1222
;; }
