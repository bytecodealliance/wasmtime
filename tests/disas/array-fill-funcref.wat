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
;; @003b                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v7 = load.i64 notrap aligned readonly can_move v52+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v8 = iadd v7, v6
;; @003b                               v9 = iconst.i64 16
;; @003b                               v10 = iadd v8, v9  ; v9 = 16
;; @003b                               v11 = load.i32 user2 readonly v10
;; @003b                               v13 = uextend.i64 v3
;; @003b                               v14 = uextend.i64 v5
;; @003b                               v16 = iadd v13, v14
;; @003b                               v12 = uextend.i64 v11
;; @003b                               v17 = icmp ugt v16, v12
;; @003b                               trapnz v17, user17
;; @003b                               v28 = load.i64 notrap aligned v52+40
;;                                     v48 = iconst.i64 20
;; @003b                               v21 = iadd v8, v48  ; v48 = 20
;;                                     v56 = iconst.i64 2
;;                                     v57 = ishl v13, v56  ; v56 = 2
;; @003b                               v24 = iadd v21, v57
;;                                     v59 = ishl v14, v56  ; v56 = 2
;; @003b                               v30 = uadd_overflow_trap v24, v59, user2
;; @003b                               v29 = iadd v7, v28
;; @003b                               v31 = icmp ugt v30, v29
;; @003b                               trapnz v31, user2
;;                                     v54 = iconst.i64 0
;; @003b                               v33 = icmp eq v14, v54  ; v54 = 0
;;                                     v47 = iconst.i64 4
;; @003b                               v32 = iadd v24, v59
;; @003b                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;; @003b                               v36 = call fn0(v0, v4)
;; @003b                               v37 = ireduce.i32 v36
;; @003b                               store user2 little v37, v34
;;                                     v61 = iconst.i64 4
;;                                     v62 = iadd v34, v61  ; v61 = 4
;; @003b                               v39 = icmp eq v62, v32
;; @003b                               brif v39, block3, block2(v62)
;;
;;                                 block3:
;; @003e                               jump block1
;;
;;                                 block1:
;; @003e                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:2 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @0049                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v8 = iadd v7, v6
;; @0049                               v9 = iconst.i64 16
;; @0049                               v10 = iadd v8, v9  ; v9 = 16
;; @0049                               v11 = load.i32 user2 readonly v10
;; @0049                               v13 = uextend.i64 v3
;; @0049                               v14 = uextend.i64 v4
;; @0049                               v16 = iadd v13, v14
;; @0049                               v12 = uextend.i64 v11
;; @0049                               v17 = icmp ugt v16, v12
;; @0049                               trapnz v17, user17
;; @0049                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @0049                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v47 = iconst.i64 2
;;                                     v48 = ishl v13, v47  ; v47 = 2
;; @0049                               v24 = iadd v21, v48
;;                                     v50 = ishl v14, v47  ; v47 = 2
;; @0049                               v30 = uadd_overflow_trap v24, v50, user2
;; @0049                               v29 = iadd v7, v28
;; @0049                               v31 = icmp ugt v30, v29
;; @0049                               trapnz v31, user2
;; @0049                               v32 = iconst.i32 0
;; @0049                               call fn0(v0, v24, v32, v50)  ; v32 = 0
;; @004c                               jump block1
;;
;;                                 block1:
;; @004c                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
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
;;                                     v62 = stack_addr.i64 ss0
;;                                     store notrap v2, v62
;; @0053                               v5 = iconst.i32 3
;; @0053                               v7 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]  ; v5 = 3
;;                                     v44 = load.i32 notrap v62
;; @0057                               trapz v44, user16
;; @0057                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @0057                               v9 = load.i64 notrap aligned readonly can_move v58+32
;; @0057                               v8 = uextend.i64 v44
;; @0057                               v10 = iadd v9, v8
;; @0057                               v11 = iconst.i64 16
;; @0057                               v12 = iadd v10, v11  ; v11 = 16
;; @0057                               v13 = load.i32 user2 readonly v12
;; @0057                               v15 = uextend.i64 v3
;; @0057                               v16 = uextend.i64 v4
;; @0057                               v18 = iadd v15, v16
;; @0057                               v14 = uextend.i64 v13
;; @0057                               v19 = icmp ugt v18, v14
;; @0057                               trapnz v19, user17
;; @0057                               v30 = load.i64 notrap aligned v58+40
;;                                     v53 = iconst.i64 20
;; @0057                               v23 = iadd v10, v53  ; v53 = 20
;;                                     v65 = iconst.i64 2
;;                                     v66 = ishl v15, v65  ; v65 = 2
;; @0057                               v26 = iadd v23, v66
;;                                     v68 = ishl v16, v65  ; v65 = 2
;; @0057                               v32 = uadd_overflow_trap v26, v68, user2
;; @0057                               v31 = iadd v9, v30
;; @0057                               v33 = icmp ugt v32, v31
;; @0057                               trapnz v33, user2
;;                                     v63 = iconst.i64 0
;; @0057                               v35 = icmp eq v16, v63  ; v63 = 0
;;                                     v52 = iconst.i64 4
;; @0057                               v34 = iadd v26, v68
;; @0057                               brif v35, block3, block2(v26)
;;
;;                                 block2(v36: i64):
;; @0057                               v38 = call fn1(v0, v7)
;; @0057                               v39 = ireduce.i32 v38
;; @0057                               store user2 little v39, v36
;;                                     v70 = iconst.i64 4
;;                                     v71 = iadd v36, v70  ; v70 = 4
;; @0057                               v41 = icmp eq v71, v34
;; @0057                               brif v41, block3, block2(v71)
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
