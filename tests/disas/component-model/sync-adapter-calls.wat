;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

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

;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1476395008 "VMGlobalImport+0x0"
;;     region4 = 402653184 "PublicGlobal"
;;     region5 = 67109000 "VMStoreContext+0x88"
;;     region6 = 1006632960 "VMDeferredThread+0x0"
;;     region7 = 1006632968 "VMDeferredThread+0x8"
;;     region8 = 1006632972 "VMDeferredThread+0xc"
;;     region9 = 1006632976 "VMDeferredThread+0x10"
;;     region10 = 67108992 "VMStoreContext+0x80"
;;     region11 = 1006632980 "VMDeferredThread+0x14"
;;     region12 = 67108996 "VMStoreContext+0x84"
;;     region13 = 1006632984 "VMDeferredThread+0x18"
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
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) tail
;;     sig3 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig3
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               jump block2
;;
;;                                 block2:
;;                                     jump block6
;;
;;                                 block6:
;; @00ee                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v9 = load.i64 notrap aligned readonly can_move region3 v3+232
;;                                     v10 = load.i32 notrap aligned region4 v9
;;                                     trapz v10, user26
;;                                     jump block9
;;
;;                                 block9:
;;                                     v11 = load.i64 notrap aligned readonly can_move region3 v3+256
;;                                     v12 = load.i32 notrap aligned region4 v11
;;                                     v8 = iconst.i32 0
;;                                     store notrap aligned region4 v8, v11  ; v8 = 0
;;                                     v20 = load.i64 notrap aligned readonly can_move region0 v3+8
;;                                     v21 = load.i64 notrap aligned region5 v20+136
;;                                     v19 = stack_addr.i64 ss0
;;                                     store notrap aligned region6 v21, v19
;;                                     v15 = iconst.i32 2
;;                                     store notrap aligned region7 v15, v19+8  ; v15 = 2
;;                                     store notrap aligned region8 v8, v19+12  ; v8 = 0
;;                                     v17 = iconst.i32 1
;;                                     store notrap aligned region9 v17, v19+16  ; v17 = 1
;;                                     v22 = load.i32 notrap aligned region10 v20+128
;;                                     store notrap aligned region11 v22, v19+20
;;                                     store notrap aligned region10 v8, v20+128  ; v8 = 0
;;                                     v24 = load.i32 notrap aligned region12 v20+132
;;                                     store notrap aligned region13 v24, v19+24
;;                                     store notrap aligned region12 v8, v20+132  ; v8 = 0
;;                                     store notrap aligned region5 v19, v20+136
;;                                     v26 = load.i64 notrap aligned readonly can_move region3 v3+208
;;                                     v27 = load.i32 notrap aligned region4 v26
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block11
;;
;;                                 block11:
;;                                     jump block12
;;
;;                                 block12:
;;                                     store.i64 notrap aligned region5 v21, v20+136
;;                                     store.i32 notrap aligned region10 v22, v20+128
;;                                     store.i32 notrap aligned region12 v24, v20+132
;;                                     jump block14
;;
;;                                 block14:
;;                                     store.i32 notrap aligned region4 v10, v9
;;                                     store.i32 notrap aligned region4 v12, v11
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
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v50 = iconst.i32 1276
;; @00f0                               return v50  ; v50 = 1276
;; }
