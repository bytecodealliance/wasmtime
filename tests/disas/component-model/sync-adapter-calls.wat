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
;; @0038                               v4 = iconst.i32 42
;;                                     v5 = iadd.i32 v2, v4  ; v4 = 42
;; @003b                               return v5
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 200 "VMContext+0xc8"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     region7 = 224 "VMContext+0xe0"
;;     region8 = 136 "VMContext+0x88"
;;     region9 = 268435592 "VMStoreContext+0x88"
;;     region10 = 3221225472 "VMDeferredThread+0x0"
;;     region11 = 3221225480 "VMDeferredThread+0x8"
;;     region12 = 3221225484 "VMDeferredThread+0xc"
;;     region13 = 3221225488 "VMDeferredThread+0x10"
;;     region14 = 268435584 "VMStoreContext+0x80"
;;     region15 = 3221225492 "VMDeferredThread+0x14"
;;     region16 = 268435588 "VMStoreContext+0x84"
;;     region17 = 3221225496 "VMDeferredThread+0x18"
;;     region18 = 176 "VMContext+0xb0"
;;     region19 = 168 "VMContext+0xa8"
;;     region20 = 152 "VMContext+0x98"
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
;;                                 block8(v9: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v14 = load.i64 notrap aligned readonly can_move region3 v4+200
;;                                     v15 = load.i32 notrap aligned region4 v14
;;                                     v16 = iconst.i32 1
;;                                     v17 = band v15, v16  ; v16 = 1
;;                                     v13 = iconst.i32 0
;;                                     v19 = icmp eq v17, v13  ; v13 = 0
;;                                     brif v19, block9, block10
;;
;;                                 block9:
;;                                     v23 = load.i64 notrap aligned readonly can_move region6 v4+88
;;                                     v22 = load.i64 notrap aligned readonly can_move region5 v4+104
;;                                     v21 = iconst.i32 23
;;                                     try_call_indirect v23(v22, v4, v21), sig1, block11, [ context v4, default: block8(exn0) ]  ; v21 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block10:
;;                                     v28 = load.i64 notrap aligned readonly can_move region7 v4+224
;;                                     v29 = load.i32 notrap aligned region4 v28
;;                                     v87 = iconst.i32 0
;;                                     store notrap aligned region4 v87, v28  ; v87 = 0
;;                                     v37 = load.i64 notrap aligned readonly can_move region0 v4+8
;;                                     v38 = load.i64 notrap aligned region9 v37+136
;;                                     v36 = stack_addr.i64 ss0
;;                                     store notrap aligned region10 v38, v36
;;                                     v32 = iconst.i32 2
;;                                     store notrap aligned region11 v32, v36+8  ; v32 = 2
;;                                     store notrap aligned region12 v87, v36+12  ; v87 = 0
;;                                     v88 = iconst.i32 1
;;                                     store notrap aligned region13 v88, v36+16  ; v88 = 1
;;                                     v39 = load.i32 notrap aligned region14 v37+128
;;                                     store notrap aligned region15 v39, v36+20
;;                                     store notrap aligned region14 v87, v37+128  ; v87 = 0
;;                                     v41 = load.i32 notrap aligned region16 v37+132
;;                                     store notrap aligned region17 v41, v36+24
;;                                     store notrap aligned region16 v87, v37+132  ; v87 = 0
;;                                     store notrap aligned region9 v36, v37+136
;;                                     v43 = load.i64 notrap aligned readonly can_move region18 v4+176
;;                                     v44 = load.i32 notrap aligned region4 v43
;;                                     v45 = iconst.i32 -2
;;                                     v46 = band v44, v45  ; v45 = -2
;;                                     store notrap aligned region4 v46, v43
;;                                     v89 = bor v44, v88  ; v88 = 1
;;                                     store notrap aligned region4 v89, v43
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
;;                                     store.i64 notrap aligned region9 v38, v37+136
;;                                     store.i32 notrap aligned region14 v39, v37+128
;;                                     store.i32 notrap aligned region16 v41, v37+132
;;                                     jump block15
;;
;;                                 block15:
;;                                     v65 = load.i32 notrap aligned region4 v14
;;                                     v90 = iconst.i32 -2
;;                                     v91 = band v65, v90  ; v90 = -2
;;                                     store notrap aligned region4 v91, v14
;;                                     v92 = iconst.i32 1
;;                                     v93 = bor v65, v92  ; v92 = 1
;;                                     store notrap aligned region4 v93, v14
;;                                     store.i32 notrap aligned region4 v29, v28
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v94 = load.i64 notrap aligned readonly can_move region6 v4+88
;;                                     v95 = load.i64 notrap aligned readonly can_move region5 v4+104
;;                                     v25 = iconst.i32 49
;;                                     call_indirect sig1, v94(v95, v4, v25)  ; v25 = 49
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
;;                                     v78 = iconst.i32 1276
;; @00f0                               return v78  ; v78 = 1276
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
;;     region9 = 3221225472 "VMDeferredThread+0x0"
;;     region10 = 3221225480 "VMDeferredThread+0x8"
;;     region11 = 3221225484 "VMDeferredThread+0xc"
;;     region12 = 3221225488 "VMDeferredThread+0x10"
;;     region13 = 268435584 "VMStoreContext+0x80"
;;     region14 = 3221225492 "VMDeferredThread+0x14"
;;     region15 = 268435588 "VMStoreContext+0x84"
;;     region16 = 3221225496 "VMDeferredThread+0x18"
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
;;                                 block6(v7: i64):
;; @00cf                               jump block3
;;
;;                                 block4:
;; @00d4                               v9 = load.i64 notrap aligned readonly can_move region2 v0+200
;; @00d4                               v10 = load.i32 notrap aligned region3 v9
;; @00d6                               v11 = iconst.i32 1
;; @00d8                               v12 = band v10, v11  ; v11 = 1
;; @00c9                               v4 = iconst.i32 0
;; @00d9                               v14 = icmp eq v12, v4  ; v4 = 0
;; @00da                               brif v14, block7, block8
;;
;;                                 block7:
;; @00de                               v18 = load.i64 notrap aligned readonly can_move region5 v0+88
;; @00de                               v17 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @00dc                               v16 = iconst.i32 23
;; @00de                               try_call_indirect v18(v17, v0, v16), sig0, block9, [ context v0, default: block6(exn0) ]  ; v16 = 23
;;
;;                                 block9:
;; @00e0                               trap user12
;;
;;                                 block8:
;; @00e2                               v19 = load.i64 notrap aligned readonly can_move region6 v0+224
;; @00e2                               v20 = load.i32 notrap aligned region3 v19
;;                                     v79 = iconst.i32 0
;; @00e8                               store notrap aligned region3 v79, v19  ; v79 = 0
;; @00f0                               v28 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @00f0                               v29 = load.i64 notrap aligned region8 v28+136
;; @00f0                               v27 = stack_addr.i64 ss0
;; @00f0                               store notrap aligned region9 v29, v27
;; @00ea                               v23 = iconst.i32 2
;; @00f0                               store notrap aligned region10 v23, v27+8  ; v23 = 2
;; @00f0                               store notrap aligned region11 v79, v27+12  ; v79 = 0
;;                                     v80 = iconst.i32 1
;; @00f0                               store notrap aligned region12 v80, v27+16  ; v80 = 1
;; @00f0                               v30 = load.i32 notrap aligned region13 v28+128
;; @00f0                               store notrap aligned region14 v30, v27+20
;; @00f0                               store notrap aligned region13 v79, v28+128  ; v79 = 0
;; @00f0                               v32 = load.i32 notrap aligned region15 v28+132
;; @00f0                               store notrap aligned region16 v32, v27+24
;; @00f0                               store notrap aligned region15 v79, v28+132  ; v79 = 0
;; @00f0                               store notrap aligned region8 v27, v28+136
;; @00f2                               v34 = load.i64 notrap aligned readonly can_move region17 v0+176
;; @00f2                               v35 = load.i32 notrap aligned region3 v34
;; @00f4                               v36 = iconst.i32 -2
;; @00f6                               v37 = band v35, v36  ; v36 = -2
;; @00f7                               store notrap aligned region3 v37, v34
;;                                     v81 = bor v35, v80  ; v80 = 1
;; @0100                               store notrap aligned region3 v81, v34
;; @0102                               jump block15
;;
;;                                 block15:
;;                                     jump block16
;;
;;                                 block16:
;;                                     jump block10
;;
;;                                 block10:
;; @0106                               jump block11
;;
;;                                 block11:
;; @0106                               store.i64 notrap aligned region8 v29, v28+136
;; @0106                               store.i32 notrap aligned region13 v30, v28+128
;; @0106                               store.i32 notrap aligned region15 v32, v28+132
;; @0106                               jump block13
;;
;;                                 block13:
;; @0108                               v56 = load.i32 notrap aligned region3 v9
;;                                     v82 = iconst.i32 -2
;;                                     v83 = band v56, v82  ; v82 = -2
;; @010d                               store notrap aligned region3 v83, v9
;;                                     v84 = iconst.i32 1
;;                                     v85 = bor v56, v84  ; v84 = 1
;; @0116                               store notrap aligned region3 v85, v9
;; @011a                               store.i32 notrap aligned region3 v20, v19
;; @011c                               jump block5
;;
;;                                 block5:
;; @011d                               jump block2
;;
;;                                 block3:
;;                                     v86 = load.i64 notrap aligned readonly can_move region5 v0+88
;;                                     v87 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0120                               v68 = iconst.i32 49
;; @0122                               call_indirect sig0, v86(v87, v0, v68)  ; v68 = 49
;; @0124                               trap user12
;;
;;                                 block2:
;; @0126                               jump block1
;;
;;                                 block1:
;;                                     v72 = iconst.i32 42
;;                                     v73 = iadd.i32 v2, v72  ; v72 = 42
;; @0126                               return v73
;; }
