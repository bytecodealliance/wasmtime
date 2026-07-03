;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i32)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const 0) (local.get $len))
  )

  (func $fill-minus-one (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const -1) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (i32.const 0xdead) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0031                               trapz v2, user16
;; @0031                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0031                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0031                               v6 = uextend.i64 v2
;; @0031                               v9 = iadd v8, v6
;; @0031                               v10 = iconst.i64 16
;; @0031                               v11 = iadd v9, v10  ; v10 = 16
;; @0031                               v12 = load.i32 user2 readonly region4 v11
;; @0031                               v14 = uextend.i64 v3
;; @0031                               v15 = uextend.i64 v5
;; @0031                               v18 = iadd v14, v15
;; @0031                               v13 = uextend.i64 v12
;; @0031                               v19 = icmp ugt v18, v13
;; @0031                               trapnz v19, user17
;; @0031                               v36 = load.i64 notrap aligned region3 v7+40
;; @0031                               v24 = iconst.i64 20
;; @0031                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @0031                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @0031                               v38 = uadd_overflow_trap v29, v52, user2
;; @0031                               v37 = iadd v8, v36
;; @0031                               v39 = icmp ugt v38, v37
;; @0031                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @0031                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0031                               v27 = iconst.i64 4
;; @0031                               v40 = iadd v29, v52
;; @0031                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;; @0031                               store.i32 user2 little region4 v4, v43
;;                                     v54 = iconst.i64 4
;;                                     v55 = iadd v43, v54  ; v54 = 4
;; @0031                               v46 = icmp eq v55, v40
;; @0031                               brif v46, block3, block2(v55)
;;
;;                                 block3:
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003f                               trapz v2, user16
;; @003f                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003f                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @003f                               v6 = uextend.i64 v2
;; @003f                               v9 = iadd v8, v6
;; @003f                               v10 = iconst.i64 16
;; @003f                               v11 = iadd v9, v10  ; v10 = 16
;; @003f                               v12 = load.i32 user2 readonly region4 v11
;; @003f                               v14 = uextend.i64 v3
;; @003f                               v15 = uextend.i64 v4
;; @003f                               v18 = iadd v14, v15
;; @003f                               v13 = uextend.i64 v12
;; @003f                               v19 = icmp ugt v18, v13
;; @003f                               trapnz v19, user17
;; @003f                               v36 = load.i64 notrap aligned region3 v7+40
;; @003f                               v24 = iconst.i64 20
;; @003f                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v14, v43  ; v43 = 2
;; @003f                               v29 = iadd v25, v44
;;                                     v46 = ishl v15, v43  ; v43 = 2
;; @003f                               v38 = uadd_overflow_trap v29, v46, user2
;; @003f                               v37 = iadd v8, v36
;; @003f                               v39 = icmp ugt v38, v37
;; @003f                               trapnz v39, user2
;; @003b                               v5 = iconst.i32 0
;; @003f                               call fn0(v0, v29, v5, v46)  ; v5 = 0
;; @0042                               jump block1
;;
;;                                 block1:
;; @0042                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004d                               trapz v2, user16
;; @004d                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004d                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @004d                               v6 = uextend.i64 v2
;; @004d                               v9 = iadd v8, v6
;; @004d                               v10 = iconst.i64 16
;; @004d                               v11 = iadd v9, v10  ; v10 = 16
;; @004d                               v12 = load.i32 user2 readonly region4 v11
;; @004d                               v14 = uextend.i64 v3
;; @004d                               v15 = uextend.i64 v4
;; @004d                               v18 = iadd v14, v15
;; @004d                               v13 = uextend.i64 v12
;; @004d                               v19 = icmp ugt v18, v13
;; @004d                               trapnz v19, user17
;; @004d                               v36 = load.i64 notrap aligned region3 v7+40
;; @004d                               v24 = iconst.i64 20
;; @004d                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v14, v43  ; v43 = 2
;; @004d                               v29 = iadd v25, v44
;;                                     v46 = ishl v15, v43  ; v43 = 2
;; @004d                               v38 = uadd_overflow_trap v29, v46, user2
;; @004d                               v37 = iadd v8, v36
;; @004d                               v39 = icmp ugt v38, v37
;; @004d                               trapnz v39, user2
;; @004d                               v40 = iconst.i32 255
;; @004d                               call fn0(v0, v29, v40, v46)  ; v40 = 255
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @005d                               trapz v2, user16
;; @005d                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @005d                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @005d                               v6 = uextend.i64 v2
;; @005d                               v9 = iadd v8, v6
;; @005d                               v10 = iconst.i64 16
;; @005d                               v11 = iadd v9, v10  ; v10 = 16
;; @005d                               v12 = load.i32 user2 readonly region4 v11
;; @005d                               v14 = uextend.i64 v3
;; @005d                               v15 = uextend.i64 v4
;; @005d                               v18 = iadd v14, v15
;; @005d                               v13 = uextend.i64 v12
;; @005d                               v19 = icmp ugt v18, v13
;; @005d                               trapnz v19, user17
;; @005d                               v36 = load.i64 notrap aligned region3 v7+40
;; @005d                               v24 = iconst.i64 20
;; @005d                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v49 = iconst.i64 2
;;                                     v50 = ishl v14, v49  ; v49 = 2
;; @005d                               v29 = iadd v25, v50
;;                                     v52 = ishl v15, v49  ; v49 = 2
;; @005d                               v38 = uadd_overflow_trap v29, v52, user2
;; @005d                               v37 = iadd v8, v36
;; @005d                               v39 = icmp ugt v38, v37
;; @005d                               trapnz v39, user2
;;                                     v47 = iconst.i64 0
;; @005d                               v42 = icmp eq v15, v47  ; v47 = 0
;; @0057                               v5 = iconst.i32 0xdead
;; @005d                               v27 = iconst.i64 4
;; @005d                               v40 = iadd v29, v52
;; @005d                               brif v42, block3, block2(v29)
;;
;;                                 block2(v43: i64):
;;                                     v54 = iconst.i32 0xdead
;; @005d                               store user2 little region4 v54, v43  ; v54 = 0xdead
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v43, v55  ; v55 = 4
;; @005d                               v46 = icmp eq v56, v40
;; @005d                               brif v46, block3, block2(v56)
;;
;;                                 block3:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
