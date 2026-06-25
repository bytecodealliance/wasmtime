;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut funcref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v funcref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null func) (local.get $len))
  )

  (func $fill-bit-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.func $hi) (local.get $len))
  )

  (func $hi)
  (elem declare func $hi)
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @003b                               trapz v2, user16
;; @003b                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v9 = iadd v8, v6
;; @003b                               v10 = iconst.i64 16
;; @003b                               v11 = iadd v9, v10  ; v10 = 16
;; @003b                               v12 = load.i32 user2 readonly region4 v11
;; @003b                               v14 = uextend.i64 v3
;; @003b                               v15 = uextend.i64 v5
;; @003b                               v18 = iadd v14, v15
;; @003b                               v13 = uextend.i64 v12
;; @003b                               v19 = icmp ugt v18, v13
;; @003b                               trapnz v19, user17
;; @003b                               v36 = load.i64 notrap aligned region3 v7+40
;; @003b                               v24 = iconst.i64 20
;; @003b                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v14, v51  ; v51 = 2
;; @003b                               v29 = iadd v25, v52
;;                                     v54 = ishl v15, v51  ; v51 = 2
;; @003b                               v38 = uadd_overflow_trap v29, v54, user2
;; @003b                               v37 = iadd v8, v36
;; @003b                               v39 = icmp ugt v38, v37
;; @003b                               trapnz v39, user2
;; @003b                               v40 = call fn0(v0, v4)
;;                                     v49 = iconst.i64 0
;; @003b                               v44 = icmp eq v15, v49  ; v49 = 0
;; @003b                               v41 = ireduce.i32 v40
;; @003b                               v27 = iconst.i64 4
;; @003b                               v42 = iadd v29, v54
;; @003b                               brif v44, block3, block2(v29)
;;
;;                                 block2(v45: i64):
;; @003b                               store.i32 notrap aligned little region4 v41, v45
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v45, v56  ; v56 = 4
;; @003b                               v48 = icmp eq v57, v42
;; @003b                               brif v48, block3, block2(v57)
;;
;;                                 block3:
;; @003e                               jump block1
;;
;;                                 block1:
;; @003e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0049                               v8 = load.i64 notrap aligned readonly can_move region2 v7+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v9 = iadd v8, v6
;; @0049                               v10 = iconst.i64 16
;; @0049                               v11 = iadd v9, v10  ; v10 = 16
;; @0049                               v12 = load.i32 user2 readonly region4 v11
;; @0049                               v14 = uextend.i64 v3
;; @0049                               v15 = uextend.i64 v4
;; @0049                               v18 = iadd v14, v15
;; @0049                               v13 = uextend.i64 v12
;; @0049                               v19 = icmp ugt v18, v13
;; @0049                               trapnz v19, user17
;; @0049                               v36 = load.i64 notrap aligned region3 v7+40
;; @0049                               v24 = iconst.i64 20
;; @0049                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v14, v50  ; v50 = 2
;; @0049                               v29 = iadd v25, v51
;;                                     v53 = ishl v15, v50  ; v50 = 2
;; @0049                               v38 = uadd_overflow_trap v29, v53, user2
;; @0049                               v37 = iadd v8, v36
;; @0049                               v39 = icmp ugt v38, v37
;; @0049                               trapnz v39, user2
;; @0045                               v5 = iconst.i64 0
;; @0049                               v40 = call fn0(v0, v5)  ; v5 = 0
;; @0049                               v44 = icmp eq v15, v5  ; v5 = 0
;; @0049                               v41 = ireduce.i32 v40
;; @0049                               v27 = iconst.i64 4
;; @0049                               v42 = iadd v29, v53
;; @0049                               brif v44, block3, block2(v29)
;;
;;                                 block2(v45: i64):
;; @0049                               store.i32 notrap aligned little region4 v41, v45
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v45, v55  ; v55 = 4
;; @0049                               v48 = icmp eq v56, v42
;; @0049                               brif v48, block3, block2(v56)
;;
;;                                 block3:
;; @004c                               jump block1
;;
;;                                 block1:
;; @004c                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v56 = stack_addr.i64 ss0
;;                                     store notrap v2, v56
;; @0053                               v5 = iconst.i32 3
;; @0053                               v6 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]  ; v5 = 3
;;                                     v55 = load.i32 notrap v56
;; @0057                               trapz v55, user16
;; @0057                               v8 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0057                               v9 = load.i64 notrap aligned readonly can_move region2 v8+32
;; @0057                               v7 = uextend.i64 v55
;; @0057                               v10 = iadd v9, v7
;; @0057                               v11 = iconst.i64 16
;; @0057                               v12 = iadd v10, v11  ; v11 = 16
;; @0057                               v13 = load.i32 user2 readonly region4 v12
;; @0057                               v15 = uextend.i64 v3
;; @0057                               v16 = uextend.i64 v4
;; @0057                               v19 = iadd v15, v16
;; @0057                               v14 = uextend.i64 v13
;; @0057                               v20 = icmp ugt v19, v14
;; @0057                               trapnz v20, user17
;; @0057                               v37 = load.i64 notrap aligned region3 v8+40
;; @0057                               v25 = iconst.i64 20
;; @0057                               v26 = iadd v10, v25  ; v25 = 20
;;                                     v59 = iconst.i64 2
;;                                     v60 = ishl v15, v59  ; v59 = 2
;; @0057                               v30 = iadd v26, v60
;;                                     v62 = ishl v16, v59  ; v59 = 2
;; @0057                               v39 = uadd_overflow_trap v30, v62, user2
;; @0057                               v38 = iadd v9, v37
;; @0057                               v40 = icmp ugt v39, v38
;; @0057                               trapnz v40, user2
;; @0057                               v41 = call fn1(v0, v6)
;;                                     v57 = iconst.i64 0
;; @0057                               v45 = icmp eq v16, v57  ; v57 = 0
;; @0057                               v42 = ireduce.i32 v41
;; @0057                               v28 = iconst.i64 4
;; @0057                               v43 = iadd v30, v62
;; @0057                               brif v45, block3, block2(v30)
;;
;;                                 block2(v46: i64):
;; @0057                               store.i32 notrap aligned little region4 v42, v46
;;                                     v64 = iconst.i64 4
;;                                     v65 = iadd v46, v64  ; v64 = 4
;; @0057                               v49 = icmp eq v65, v43
;; @0057                               brif v49, block3, block2(v65)
;;
;;                                 block3:
;; @005a                               jump block1
;;
;;                                 block1:
;; @005a                               return
;; }
;;
;; function u0:3(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
