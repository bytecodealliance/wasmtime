;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
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
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 200 "VMContext+0xc8"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 224 "VMContext+0xe0"
;;     region6 = 136 "VMContext+0x88"
;;     region7 = 268435592 "VMStoreContext+0x88"
;;     region8 = 4026531840 "VMDeferredThread+0x0"
;;     region9 = 4026531848 "VMDeferredThread+0x8"
;;     region10 = 4026531852 "VMDeferredThread+0xc"
;;     region11 = 4026531856 "VMDeferredThread+0x10"
;;     region12 = 268435584 "VMStoreContext+0x80"
;;     region13 = 4026531860 "VMDeferredThread+0x14"
;;     region14 = 268435588 "VMStoreContext+0x84"
;;     region15 = 4026531864 "VMDeferredThread+0x18"
;;     region16 = 176 "VMContext+0xb0"
;;     region17 = 168 "VMContext+0xa8"
;;     region18 = 152 "VMContext+0x98"
;;     region19 = 104 "VMContext+0x68"
;;     region20 = 88 "VMContext+0x58"
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
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     sig2 = (i64 vmctx, i64, i32, i32, i32) tail
;;     sig3 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig4 = (i64 vmctx, i64) tail
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
;;                                 block8(v5: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v9 = load.i64 notrap aligned readonly can_move region3 v3+200
;;                                     v10 = load.i32 notrap aligned region4 v9
;;                                     brif v10, block9, block10
;;
;;                                 block10:
;;                                     v53 = load.i64 notrap aligned readonly can_move region20 v3+88
;;                                     v52 = load.i64 notrap aligned readonly can_move region19 v3+104
;;                                     v51 = iconst.i32 23
;;                                     try_call_indirect v53(v52, v3, v51), sig1, block11, [ context v3, default: block8(exn0) ]  ; v51 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block9:
;;                                     v11 = load.i64 notrap aligned readonly can_move region5 v3+224
;;                                     v12 = load.i32 notrap aligned region4 v11
;;                                     v8 = iconst.i32 0
;;                                     store notrap aligned region4 v8, v11  ; v8 = 0
;;                                     v20 = load.i64 notrap aligned readonly can_move region0 v3+8
;;                                     v21 = load.i64 notrap aligned region7 v20+136
;;                                     v19 = stack_addr.i64 ss0
;;                                     store notrap aligned region8 v21, v19
;;                                     v15 = iconst.i32 2
;;                                     store notrap aligned region9 v15, v19+8  ; v15 = 2
;;                                     store notrap aligned region10 v8, v19+12  ; v8 = 0
;;                                     v17 = iconst.i32 1
;;                                     store notrap aligned region11 v17, v19+16  ; v17 = 1
;;                                     v22 = load.i32 notrap aligned region12 v20+128
;;                                     store notrap aligned region13 v22, v19+20
;;                                     store notrap aligned region12 v8, v20+128  ; v8 = 0
;;                                     v24 = load.i32 notrap aligned region14 v20+132
;;                                     store notrap aligned region15 v24, v19+24
;;                                     store notrap aligned region14 v8, v20+132  ; v8 = 0
;;                                     store notrap aligned region7 v19, v20+136
;;                                     v26 = load.i64 notrap aligned readonly can_move region16 v3+176
;;                                     v27 = load.i32 notrap aligned region4 v26
;;                                     store notrap aligned region4 v8, v26  ; v8 = 0
;;                                     store notrap aligned region4 v27, v26
;;                                     jump block17
;;
;;                                 block17:
;;                                     jump block18
;;
;;                                 block18:
;;                                     jump block12
;;
;;                                 block12:
;;                                     jump block13
;;
;;                                 block13:
;;                                     store.i64 notrap aligned region7 v21, v20+136
;;                                     store.i32 notrap aligned region12 v22, v20+128
;;                                     store.i32 notrap aligned region14 v24, v20+132
;;                                     jump block15
;;
;;                                 block15:
;;                                     v61 = iconst.i32 0
;;                                     store notrap aligned region4 v61, v9  ; v61 = 0
;;                                     store.i32 notrap aligned region4 v10, v9
;;                                     store.i32 notrap aligned region4 v12, v11
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v62 = load.i64 notrap aligned readonly can_move region20 v3+88
;;                                     v63 = load.i64 notrap aligned readonly can_move region19 v3+104
;;                                     v48 = iconst.i32 49
;;                                     call_indirect sig1, v62(v63, v3, v48)  ; v48 = 49
;;                                     trap user12
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block19
;;
;;                                 block19:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v54 = iconst.i32 1276
;; @00f0                               return v54  ; v54 = 1276
;; }
;;
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 200 "VMContext+0xc8"
;;     region3 = 1610612736 "PublicGlobal"
;;     region4 = 104 "VMContext+0x68"
;;     region5 = 88 "VMContext+0x58"
;;     region6 = 224 "VMContext+0xe0"
;;     region7 = 136 "VMContext+0x88"
;;     region8 = 268435592 "VMStoreContext+0x88"
;;     region9 = 4026531840 "VMDeferredThread+0x0"
;;     region10 = 4026531848 "VMDeferredThread+0x8"
;;     region11 = 4026531852 "VMDeferredThread+0xc"
;;     region12 = 4026531856 "VMDeferredThread+0x10"
;;     region13 = 268435584 "VMStoreContext+0x80"
;;     region14 = 4026531860 "VMDeferredThread+0x14"
;;     region15 = 268435588 "VMStoreContext+0x84"
;;     region16 = 4026531864 "VMDeferredThread+0x18"
;;     region17 = 176 "VMContext+0xb0"
;;     region18 = 72 "VMContext+0x48"
;;     region19 = 168 "VMContext+0xa8"
;;     region20 = 152 "VMContext+0x98"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32, i32, i32) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig3 = (i64 vmctx, i64) tail
;;     fn0 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @00cf                               jump block4
;;
;;                                 block6(v4: i64):
;; @00cf                               jump block3
;;
;;                                 block4:
;; @00d6                               v6 = load.i64 notrap aligned readonly can_move region2 v0+200
;; @00d6                               v7 = load.i32 notrap aligned region3 v6
;; @00da                               brif v7, block7, block8
;;
;;                                 block8:
;; @00de                               v10 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @00de                               v9 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @00dc                               v8 = iconst.i32 23
;; @00de                               try_call_indirect v10(v9, v0, v8), sig0, block9, [ context v0, default: block6(exn0) ]  ; v8 = 23
;;
;;                                 block9:
;; @00e0                               trap user12
;;
;;                                 block7:
;; @00e2                               v11 = load.i64 notrap aligned readonly can_move region6 v0+224
;; @00e2                               v12 = load.i32 notrap aligned region3 v11
;; @00c9                               v3 = iconst.i32 0
;; @00e8                               store notrap aligned region3 v3, v11  ; v3 = 0
;; @00f0                               v20 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @00f0                               v21 = load.i64 notrap aligned region8 v20+136
;; @00f0                               v19 = stack_addr.i64 ss0
;; @00f0                               store notrap aligned region9 v21, v19
;; @00ea                               v15 = iconst.i32 2
;; @00f0                               store notrap aligned region10 v15, v19+8  ; v15 = 2
;; @00f0                               store notrap aligned region11 v3, v19+12  ; v3 = 0
;; @00ee                               v17 = iconst.i32 1
;; @00f0                               store notrap aligned region12 v17, v19+16  ; v17 = 1
;; @00f0                               v22 = load.i32 notrap aligned region13 v20+128
;; @00f0                               store notrap aligned region14 v22, v19+20
;; @00f0                               store notrap aligned region13 v3, v20+128  ; v3 = 0
;; @00f0                               v24 = load.i32 notrap aligned region15 v20+132
;; @00f0                               store notrap aligned region16 v24, v19+24
;; @00f0                               store notrap aligned region15 v3, v20+132  ; v3 = 0
;; @00f0                               store notrap aligned region8 v19, v20+136
;; @00f2                               v26 = load.i64 notrap aligned readonly can_move region17 v0+176
;; @00f2                               v27 = load.i32 notrap aligned region3 v26
;; @00f8                               store notrap aligned region3 v3, v26  ; v3 = 0
;; @00fe                               store notrap aligned region3 v27, v26
;; @0100                               jump block15
;;
;;                                 block15:
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block10
;;
;;                                 block10:
;; @0104                               jump block11
;;
;;                                 block11:
;; @0104                               store.i64 notrap aligned region8 v21, v20+136
;; @0104                               store.i32 notrap aligned region13 v22, v20+128
;; @0104                               store.i32 notrap aligned region15 v24, v20+132
;; @0104                               jump block13
;;
;;                                 block13:
;;                                     v55 = iconst.i32 0
;; @0108                               store notrap aligned region3 v55, v6  ; v55 = 0
;; @010e                               store.i32 notrap aligned region3 v7, v6
;; @0112                               store.i32 notrap aligned region3 v12, v11
;; @0114                               jump block5
;;
;;                                 block5:
;; @0115                               jump block2
;;
;;                                 block3:
;;                                     v56 = load.i64 notrap aligned readonly can_move region5 v0+88
;;                                     v57 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0118                               v49 = iconst.i32 49
;; @011a                               call_indirect sig0, v56(v57, v0, v49)  ; v49 = 49
;; @011c                               trap user12
;;
;;                                 block2:
;; @011e                               jump block1
;;
;;                                 block1:
;;                                     v52 = iconst.i32 42
;;                                     v53 = iadd.i32 v2, v52  ; v52 = 42
;; @011e                               return v53
;; }
