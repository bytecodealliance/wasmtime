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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @003b                               trapz v2, user16
;; @003b                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v7 = load.i64 notrap aligned readonly can_move v51+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v8 = iadd v7, v6
;; @003b                               v9 = iconst.i64 16
;; @003b                               v10 = iadd v8, v9  ; v9 = 16
;; @003b                               v11 = load.i32 user2 readonly region0 v10
;; @003b                               v13 = uextend.i64 v3
;; @003b                               v14 = uextend.i64 v5
;; @003b                               v17 = iadd v13, v14
;; @003b                               v12 = uextend.i64 v11
;; @003b                               v18 = icmp ugt v17, v12
;; @003b                               trapnz v18, user17
;; @003b                               v32 = load.i64 notrap aligned v51+40
;; @003b                               v22 = iconst.i64 20
;; @003b                               v23 = iadd v8, v22  ; v22 = 20
;;                                     v55 = iconst.i64 2
;;                                     v56 = ishl v13, v55  ; v55 = 2
;; @003b                               v27 = iadd v23, v56
;;                                     v58 = ishl v14, v55  ; v55 = 2
;; @003b                               v34 = uadd_overflow_trap v27, v58, user2
;; @003b                               v33 = iadd v7, v32
;; @003b                               v35 = icmp ugt v34, v33
;; @003b                               trapnz v35, user2
;; @003b                               v36 = call fn0(v0, v4)
;;                                     v53 = iconst.i64 0
;; @003b                               v40 = icmp eq v14, v53  ; v53 = 0
;; @003b                               v37 = ireduce.i32 v36
;; @003b                               v25 = iconst.i64 4
;; @003b                               v38 = iadd v27, v58
;; @003b                               brif v40, block3, block2(v27)
;;
;;                                 block2(v41: i64):
;; @003b                               store.i32 notrap aligned little v37, v41
;;                                     v60 = iconst.i64 4
;;                                     v61 = iadd v41, v60  ; v60 = 4
;; @003b                               v44 = icmp eq v61, v38
;; @003b                               brif v44, block3, block2(v61)
;;
;;                                 block3:
;; @003e                               jump block1
;;
;;                                 block1:
;; @003e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @0049                               v7 = load.i64 notrap aligned readonly can_move v51+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v8 = iadd v7, v6
;; @0049                               v9 = iconst.i64 16
;; @0049                               v10 = iadd v8, v9  ; v9 = 16
;; @0049                               v11 = load.i32 user2 readonly region0 v10
;; @0049                               v13 = uextend.i64 v3
;; @0049                               v14 = uextend.i64 v4
;; @0049                               v17 = iadd v13, v14
;; @0049                               v12 = uextend.i64 v11
;; @0049                               v18 = icmp ugt v17, v12
;; @0049                               trapnz v18, user17
;; @0049                               v32 = load.i64 notrap aligned v51+40
;; @0049                               v22 = iconst.i64 20
;; @0049                               v23 = iadd v8, v22  ; v22 = 20
;;                                     v54 = iconst.i64 2
;;                                     v55 = ishl v13, v54  ; v54 = 2
;; @0049                               v27 = iadd v23, v55
;;                                     v57 = ishl v14, v54  ; v54 = 2
;; @0049                               v34 = uadd_overflow_trap v27, v57, user2
;; @0049                               v33 = iadd v7, v32
;; @0049                               v35 = icmp ugt v34, v33
;; @0049                               trapnz v35, user2
;; @0045                               v5 = iconst.i64 0
;; @0049                               v36 = call fn0(v0, v5)  ; v5 = 0
;; @0049                               v40 = icmp eq v14, v5  ; v5 = 0
;; @0049                               v37 = ireduce.i32 v36
;; @0049                               v25 = iconst.i64 4
;; @0049                               v38 = iadd v27, v57
;; @0049                               brif v40, block3, block2(v27)
;;
;;                                 block2(v41: i64):
;; @0049                               store.i32 notrap aligned little v37, v41
;;                                     v59 = iconst.i64 4
;;                                     v60 = iadd v41, v59  ; v59 = 4
;; @0049                               v44 = icmp eq v60, v38
;; @0049                               brif v44, block3, block2(v60)
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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:25 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v52 = stack_addr.i64 ss0
;;                                     store notrap v2, v52
;; @0053                               v5 = iconst.i32 3
;; @0053                               v6 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]  ; v5 = 3
;;                                     v51 = load.i32 notrap v52
;; @0057                               trapz v51, user16
;; @0057                               v59 = load.i64 notrap aligned readonly can_move v0+8
;; @0057                               v8 = load.i64 notrap aligned readonly can_move v59+32
;; @0057                               v7 = uextend.i64 v51
;; @0057                               v9 = iadd v8, v7
;; @0057                               v10 = iconst.i64 16
;; @0057                               v11 = iadd v9, v10  ; v10 = 16
;; @0057                               v12 = load.i32 user2 readonly region0 v11
;; @0057                               v14 = uextend.i64 v3
;; @0057                               v15 = uextend.i64 v4
;; @0057                               v18 = iadd v14, v15
;; @0057                               v13 = uextend.i64 v12
;; @0057                               v19 = icmp ugt v18, v13
;; @0057                               trapnz v19, user17
;; @0057                               v33 = load.i64 notrap aligned v59+40
;; @0057                               v23 = iconst.i64 20
;; @0057                               v24 = iadd v9, v23  ; v23 = 20
;;                                     v63 = iconst.i64 2
;;                                     v64 = ishl v14, v63  ; v63 = 2
;; @0057                               v28 = iadd v24, v64
;;                                     v66 = ishl v15, v63  ; v63 = 2
;; @0057                               v35 = uadd_overflow_trap v28, v66, user2
;; @0057                               v34 = iadd v8, v33
;; @0057                               v36 = icmp ugt v35, v34
;; @0057                               trapnz v36, user2
;; @0057                               v37 = call fn1(v0, v6)
;;                                     v61 = iconst.i64 0
;; @0057                               v41 = icmp eq v15, v61  ; v61 = 0
;; @0057                               v38 = ireduce.i32 v37
;; @0057                               v26 = iconst.i64 4
;; @0057                               v39 = iadd v28, v66
;; @0057                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;; @0057                               store.i32 notrap aligned little v38, v42
;;                                     v68 = iconst.i64 4
;;                                     v69 = iadd v42, v68  ; v68 = 4
;; @0057                               v45 = icmp eq v69, v39
;; @0057                               brif v45, block3, block2(v69)
;;
;;                                 block3:
;; @005a                               jump block1
;;
;;                                 block1:
;; @005a                               return
;; }
;;
;; function u0:3(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return
;; }
