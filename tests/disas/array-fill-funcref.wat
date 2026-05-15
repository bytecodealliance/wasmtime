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
;;     fn0 = colocated u805306368:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @003b                               trapz v2, user16
;; @003b                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @003b                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @003b                               v6 = uextend.i64 v2
;; @003b                               v8 = iadd v7, v6
;; @003b                               v9 = iconst.i64 24
;; @003b                               v10 = iadd v8, v9  ; v9 = 24
;; @003b                               v11 = load.i32 user2 readonly v10
;; @003b                               v12 = uadd_overflow_trap v3, v5, user17
;; @003b                               v13 = icmp ugt v12, v11
;; @003b                               trapnz v13, user17
;; @003b                               v15 = uextend.i64 v11
;;                                     v46 = iconst.i64 2
;;                                     v47 = ishl v15, v46  ; v46 = 2
;;                                     v43 = iconst.i64 32
;; @003b                               v17 = ushr v47, v43  ; v43 = 32
;; @003b                               trapnz v17, user2
;;                                     v56 = iconst.i32 2
;;                                     v57 = ishl v11, v56  ; v56 = 2
;; @003b                               v19 = iconst.i32 28
;; @003b                               v20 = uadd_overflow_trap v57, v19, user2  ; v19 = 28
;; @003b                               v24 = uadd_overflow_trap v2, v20, user2
;; @003b                               v25 = uextend.i64 v24
;; @003b                               v27 = iadd v7, v25
;;                                     v63 = ishl v3, v56  ; v56 = 2
;;                                     v65 = iadd v63, v19  ; v19 = 28
;; @003b                               v28 = isub v20, v65
;; @003b                               v29 = uextend.i64 v28
;; @003b                               v30 = isub v27, v29
;;                                     v67 = ishl v5, v56  ; v56 = 2
;; @003b                               v32 = uextend.i64 v67
;; @003b                               v33 = iadd v30, v32
;; @003b                               v14 = iconst.i64 4
;; @003b                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @003b                               v36 = icmp eq v35, v33
;; @003b                               brif v36, block4, block3
;;
;;                                 block3:
;; @003b                               v38 = call fn0(v0, v4)
;; @003b                               v39 = ireduce.i32 v38
;; @003b                               store user2 little v39, v35
;;                                     v69 = iconst.i64 4
;;                                     v70 = iadd.i64 v35, v69  ; v69 = 4
;; @003b                               jump block2(v70)
;;
;;                                 block4:
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
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               trapz v2, user16
;; @0049                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0049                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0049                               v6 = uextend.i64 v2
;; @0049                               v8 = iadd v7, v6
;; @0049                               v9 = iconst.i64 24
;; @0049                               v10 = iadd v8, v9  ; v9 = 24
;; @0049                               v11 = load.i32 user2 readonly v10
;; @0049                               v12 = uadd_overflow_trap v3, v4, user17
;; @0049                               v13 = icmp ugt v12, v11
;; @0049                               trapnz v13, user17
;; @0049                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v15, v51  ; v51 = 2
;;                                     v48 = iconst.i64 32
;; @0049                               v17 = ushr v52, v48  ; v48 = 32
;; @0049                               trapnz v17, user2
;;                                     v61 = iconst.i32 2
;;                                     v62 = ishl v11, v61  ; v61 = 2
;; @0049                               v19 = iconst.i32 28
;; @0049                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 28
;; @0049                               v24 = uadd_overflow_trap v2, v20, user2
;; @0049                               v37 = load.i64 notrap aligned v49+40
;; @0049                               v25 = uextend.i64 v24
;; @0049                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 2
;;                                     v70 = iadd v68, v19  ; v19 = 28
;; @0049                               v28 = isub v20, v70
;; @0049                               v29 = uextend.i64 v28
;; @0049                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 2
;; @0049                               v32 = uextend.i64 v72
;; @0049                               v33 = iadd v30, v32
;; @0049                               v38 = iadd v7, v37
;; @0049                               v39 = icmp ugt v33, v38
;; @0049                               trapnz v39, user2
;; @0049                               v35 = iconst.i32 0
;; @0049                               call fn0(v0, v30, v35, v32)  ; v35 = 0
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
;;     fn0 = colocated u805306368:7 sig0
;;     fn1 = colocated u805306368:28 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v2, v54
;; @0053                               v5 = iconst.i32 3
;; @0053                               v7 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]  ; v5 = 3
;;                                     v45 = load.i32 notrap v54
;; @0057                               trapz v45, user16
;; @0057                               v50 = load.i64 notrap aligned readonly can_move v0+8
;; @0057                               v9 = load.i64 notrap aligned readonly can_move v50+32
;; @0057                               v8 = uextend.i64 v45
;; @0057                               v10 = iadd v9, v8
;; @0057                               v11 = iconst.i64 24
;; @0057                               v12 = iadd v10, v11  ; v11 = 24
;; @0057                               v13 = load.i32 user2 readonly v12
;; @0057                               v14 = uadd_overflow_trap v3, v4, user17
;; @0057                               v15 = icmp ugt v14, v13
;; @0057                               trapnz v15, user17
;; @0057                               v17 = uextend.i64 v13
;;                                     v55 = iconst.i64 2
;;                                     v56 = ishl v17, v55  ; v55 = 2
;;                                     v49 = iconst.i64 32
;; @0057                               v19 = ushr v56, v49  ; v49 = 32
;; @0057                               trapnz v19, user2
;;                                     v65 = iconst.i32 2
;;                                     v66 = ishl v13, v65  ; v65 = 2
;; @0057                               v21 = iconst.i32 28
;; @0057                               v22 = uadd_overflow_trap v66, v21, user2  ; v21 = 28
;; @0057                               v26 = uadd_overflow_trap v45, v22, user2
;; @0057                               v27 = uextend.i64 v26
;; @0057                               v29 = iadd v9, v27
;;                                     v72 = ishl v3, v65  ; v65 = 2
;;                                     v74 = iadd v72, v21  ; v21 = 28
;; @0057                               v30 = isub v22, v74
;; @0057                               v31 = uextend.i64 v30
;; @0057                               v32 = isub v29, v31
;;                                     v76 = ishl v4, v65  ; v65 = 2
;; @0057                               v34 = uextend.i64 v76
;; @0057                               v35 = iadd v32, v34
;; @0057                               v16 = iconst.i64 4
;; @0057                               jump block2(v32)
;;
;;                                 block2(v37: i64):
;; @0057                               v38 = icmp eq v37, v35
;; @0057                               brif v38, block4, block3
;;
;;                                 block3:
;; @0057                               v40 = call fn1(v0, v7)
;; @0057                               v41 = ireduce.i32 v40
;; @0057                               store user2 little v41, v37
;;                                     v78 = iconst.i64 4
;;                                     v79 = iadd.i64 v37, v78  ; v78 = 4
;; @0057                               jump block2(v79)
;;
;;                                 block4:
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
