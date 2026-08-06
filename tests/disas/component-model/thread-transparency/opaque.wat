;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

;; The callee here imports `canon context.get`, so the adapter is *not*
;; thread-transparent and the `enter-sync-call`/`exit-sync-call` calls must
;; still be emitted.

(component
  (component $A
    (core func $cget (canon context.get i32 0))

    (core module $M
      (import "" "cget" (func $cget (result i32)))
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (call $cget))
      )
    )

    (core instance $m
      (instantiate $M
        (with "" (instance (export "cget" (func $cget))))
      )
    )

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
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 24, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 738197568 "VMComponentContext+0x40"
;;     region5 = 67109000 "VMStoreContext+0x88"
;;     region6 = 1006632960 "VMDeferredThread+0x0"
;;     region7 = 1006632968 "VMDeferredThread+0x8"
;;     region8 = 1006632972 "VMDeferredThread+0xc"
;;     region9 = 67108992 "VMStoreContext+0x80"
;;     region10 = 1006632976 "VMDeferredThread+0x10"
;;     region11 = 67108996 "VMStoreContext+0x84"
;;     region12 = 1006632980 "VMDeferredThread+0x14"
;;     region13 = 738197552 "VMComponentContext+0x30"
;;     region14 = 1207959560 "VMFunctionImport+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64, i32, i32) tail
;;     sig3 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig4 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig3
;;     fn2 = colocated u2147483648:17 sig4
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0123                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @0123                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v9 = load.i64 notrap aligned readonly can_move region3 v3+232
;;                                     v10 = load.i32 notrap aligned region4 v9
;;                                     trapz v10, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v15 = load.i64 notrap aligned readonly can_move region0 v3+8
;;                                     v16 = load.i64 notrap aligned region5 v15+136
;;                                     v14 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v16, v14
;;                                     v8 = iconst.i32 0
;;                                     store notrap aligned region7 v8, v14+8  ; v8 = 0
;;                                     v12 = iconst.i32 1
;;                                     store notrap aligned region8 v12, v14+12  ; v12 = 1
;;                                     v17 = load.i32 notrap aligned region9 v15+128
;;                                     store notrap aligned region10 v17, v14+16
;;                                     store notrap aligned region9 v8, v15+128  ; v8 = 0
;;                                     v19 = load.i32 notrap aligned region11 v15+132
;;                                     store notrap aligned region12 v19, v14+20
;;                                     store notrap aligned region11 v8, v15+132  ; v8 = 0
;;                                     store notrap aligned region5 v14, v15+136
;;                                     v21 = load.i64 notrap aligned readonly can_move region3 v3+208
;;                                     v22 = load.i32 notrap aligned region13 v21
;;                                     jump block16
;;
;;                                 block16:
;;                                     v26 = load.i64 notrap aligned readonly can_move region2 v3+72
;;                                     v28 = load.i64 notrap aligned readonly can_move region0 v26+8
;;                                     v29 = load.i32 notrap aligned region9 v28+128
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block12
;;
;;                                 block12:
;;                                     store.i64 notrap aligned region5 v16, v15+136
;;                                     store.i32 notrap aligned region9 v17, v15+128
;;                                     store.i32 notrap aligned region11 v19, v15+132
;;                                     jump block14
;;
;;                                 block14:
;;                                     store.i32 notrap aligned region4 v10, v9
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block18
;;
;;                                 block18:
;; @0125                               jump block1
;;
;;                                 block1:
;; @0120                               v2 = iconst.i32 1234
;;                                     v46 = iadd.i32 v29, v2  ; v2 = 1234
;; @0125                               return v46
;; }
