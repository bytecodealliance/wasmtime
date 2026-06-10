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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @003b                               trapz v2, user16
;; @003b                               v48 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003b                               v7 = load.i64 notrap aligned readonly can_move v48+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v8 = iadd v7, v6
;; @003b                               v9 = iconst.i64 16
;; @003b                               v10 = iadd v8, v9  ; v9 = 16
;; @003b                               v11 = load.i32 user2 readonly region1 v10
;; @003b                               v13 = uextend.i64 v3
;; @003b                               v14 = uextend.i64 v5
;; @003b                               v17 = iadd v13, v14
;; @003b                               v12 = uextend.i64 v11
;; @003b                               v18 = icmp ugt v17, v12
;; @003b                               trapnz v18, user17
;; @003b                               v35 = load.i64 notrap aligned v48+40
;; @003b                               v23 = iconst.i64 20
;; @003b                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v52 = iconst.i64 2
;;                                     v53 = ishl v13, v52  ; v52 = 2
;; @003b                               v28 = iadd v24, v53
;;                                     v55 = ishl v14, v52  ; v52 = 2
;; @003b                               v37 = uadd_overflow_trap v28, v55, user2
;; @003b                               v36 = iadd v7, v35
;; @003b                               v38 = icmp ugt v37, v36
;; @003b                               trapnz v38, user2
;; @003b                               v39 = call fn0(v0, v4)
;;                                     v50 = iconst.i64 0
;; @003b                               v43 = icmp eq v14, v50  ; v50 = 0
;; @003b                               v40 = ireduce.i32 v39
;; @003b                               v26 = iconst.i64 4
;; @003b                               v41 = iadd v28, v55
;; @003b                               brif v43, block3, block2(v28)
;;
;;                                 block2(v44: i64):
;; @003b                               store.i32 notrap aligned little v40, v44
;;                                     v57 = iconst.i64 4
;;                                     v58 = iadd v44, v57  ; v57 = 4
;; @003b                               v47 = icmp eq v58, v41
;; @003b                               brif v47, block3, block2(v58)
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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v48 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0049                               v7 = load.i64 notrap aligned readonly can_move v48+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v8 = iadd v7, v6
;; @0049                               v9 = iconst.i64 16
;; @0049                               v10 = iadd v8, v9  ; v9 = 16
;; @0049                               v11 = load.i32 user2 readonly region1 v10
;; @0049                               v13 = uextend.i64 v3
;; @0049                               v14 = uextend.i64 v4
;; @0049                               v17 = iadd v13, v14
;; @0049                               v12 = uextend.i64 v11
;; @0049                               v18 = icmp ugt v17, v12
;; @0049                               trapnz v18, user17
;; @0049                               v35 = load.i64 notrap aligned v48+40
;; @0049                               v23 = iconst.i64 20
;; @0049                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v13, v51  ; v51 = 2
;; @0049                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 2
;; @0049                               v37 = uadd_overflow_trap v28, v54, user2
;; @0049                               v36 = iadd v7, v35
;; @0049                               v38 = icmp ugt v37, v36
;; @0049                               trapnz v38, user2
;; @0045                               v5 = iconst.i64 0
;; @0049                               v39 = call fn0(v0, v5)  ; v5 = 0
;; @0049                               v43 = icmp eq v14, v5  ; v5 = 0
;; @0049                               v40 = ireduce.i32 v39
;; @0049                               v26 = iconst.i64 4
;; @0049                               v41 = iadd v28, v54
;; @0049                               brif v43, block3, block2(v28)
;;
;;                                 block2(v44: i64):
;; @0049                               store.i32 notrap aligned little v40, v44
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v44, v56  ; v56 = 4
;; @0049                               v47 = icmp eq v57, v41
;; @0049                               brif v47, block3, block2(v57)
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
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v55 = stack_addr.i64 ss0
;;                                     store notrap v2, v55
;; @0053                               v5 = iconst.i32 3
;; @0053                               v6 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]  ; v5 = 3
;;                                     v54 = load.i32 notrap v55
;; @0057                               trapz v54, user16
;; @0057                               v56 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0057                               v8 = load.i64 notrap aligned readonly can_move v56+32
;; @0057                               v7 = uextend.i64 v54
;; @0057                               v9 = iadd v8, v7
;; @0057                               v10 = iconst.i64 16
;; @0057                               v11 = iadd v9, v10  ; v10 = 16
;; @0057                               v12 = load.i32 user2 readonly region1 v11
;; @0057                               v14 = uextend.i64 v3
;; @0057                               v15 = uextend.i64 v4
;; @0057                               v18 = iadd v14, v15
;; @0057                               v13 = uextend.i64 v12
;; @0057                               v19 = icmp ugt v18, v13
;; @0057                               trapnz v19, user17
;; @0057                               v36 = load.i64 notrap aligned v56+40
;; @0057                               v24 = iconst.i64 20
;; @0057                               v25 = iadd v9, v24  ; v24 = 20
;;                                     v60 = iconst.i64 2
;;                                     v61 = ishl v14, v60  ; v60 = 2
;; @0057                               v29 = iadd v25, v61
;;                                     v63 = ishl v15, v60  ; v60 = 2
;; @0057                               v38 = uadd_overflow_trap v29, v63, user2
;; @0057                               v37 = iadd v8, v36
;; @0057                               v39 = icmp ugt v38, v37
;; @0057                               trapnz v39, user2
;; @0057                               v40 = call fn1(v0, v6)
;;                                     v58 = iconst.i64 0
;; @0057                               v44 = icmp eq v15, v58  ; v58 = 0
;; @0057                               v41 = ireduce.i32 v40
;; @0057                               v27 = iconst.i64 4
;; @0057                               v42 = iadd v29, v63
;; @0057                               brif v44, block3, block2(v29)
;;
;;                                 block2(v45: i64):
;; @0057                               store.i32 notrap aligned little v41, v45
;;                                     v65 = iconst.i64 4
;;                                     v66 = iadd v45, v65  ; v65 = 4
;; @0057                               v48 = icmp eq v66, v42
;; @0057                               brif v48, block3, block2(v66)
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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
