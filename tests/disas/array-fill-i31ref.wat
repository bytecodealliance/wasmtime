;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut i31ref)))

  (func $fill-i31thing (param $a (ref $a)) (param $i i32) (param $v i31ref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null i31) (local.get $len))
  )

  (func $fill-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.i31 (i32.const -1)) (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly region1 v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v17 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v18 = icmp ugt v17, v12
;; @0030                               trapnz v18, user17
;; @0030                               v35 = load.i64 notrap aligned v46+40
;; @0030                               v23 = iconst.i64 20
;; @0030                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v13, v50  ; v50 = 2
;; @0030                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 2
;; @0030                               v37 = uadd_overflow_trap v28, v53, user2
;; @0030                               v36 = iadd v7, v35
;; @0030                               v38 = icmp ugt v37, v36
;; @0030                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @0030                               v41 = icmp eq v14, v48  ; v48 = 0
;; @0030                               v26 = iconst.i64 4
;; @0030                               v39 = iadd v28, v53
;; @0030                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;; @0030                               store.i32 user2 little region1 v4, v42
;;                                     v55 = iconst.i64 4
;;                                     v56 = iadd v42, v55  ; v55 = 4
;; @0030                               v45 = icmp eq v56, v39
;; @0030                               brif v45, block3, block2(v56)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003e                               trapz v2, user16
;; @003e                               v46 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003e                               v7 = load.i64 notrap aligned readonly can_move v46+32
;; @003e                               v6 = uextend.i64 v2
;; @003e                               v8 = iadd v7, v6
;; @003e                               v9 = iconst.i64 16
;; @003e                               v10 = iadd v8, v9  ; v9 = 16
;; @003e                               v11 = load.i32 user2 readonly region1 v10
;; @003e                               v13 = uextend.i64 v3
;; @003e                               v14 = uextend.i64 v4
;; @003e                               v17 = iadd v13, v14
;; @003e                               v12 = uextend.i64 v11
;; @003e                               v18 = icmp ugt v17, v12
;; @003e                               trapnz v18, user17
;; @003e                               v35 = load.i64 notrap aligned v46+40
;; @003e                               v23 = iconst.i64 20
;; @003e                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v13, v50  ; v50 = 2
;; @003e                               v28 = iadd v24, v51
;;                                     v53 = ishl v14, v50  ; v50 = 2
;; @003e                               v37 = uadd_overflow_trap v28, v53, user2
;; @003e                               v36 = iadd v7, v35
;; @003e                               v38 = icmp ugt v37, v36
;; @003e                               trapnz v38, user2
;;                                     v48 = iconst.i64 0
;; @003e                               v41 = icmp eq v14, v48  ; v48 = 0
;; @003a                               v5 = iconst.i32 0
;; @003e                               v26 = iconst.i64 4
;; @003e                               v39 = iadd v28, v53
;; @003e                               brif v41, block3, block2(v28)
;;
;;                                 block2(v42: i64):
;;                                     v55 = iconst.i32 0
;; @003e                               store user2 little region1 v55, v42  ; v55 = 0
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v42, v56  ; v56 = 4
;; @003e                               v45 = icmp eq v57, v39
;; @003e                               brif v45, block3, block2(v57)
;;
;;                                 block3:
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004e                               trapz v2, user16
;; @004e                               v50 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004e                               v11 = load.i64 notrap aligned readonly can_move v50+32
;; @004e                               v10 = uextend.i64 v2
;; @004e                               v12 = iadd v11, v10
;; @004e                               v13 = iconst.i64 16
;; @004e                               v14 = iadd v12, v13  ; v13 = 16
;; @004e                               v15 = load.i32 user2 readonly region1 v14
;; @004e                               v17 = uextend.i64 v3
;; @004e                               v18 = uextend.i64 v4
;; @004e                               v21 = iadd v17, v18
;; @004e                               v16 = uextend.i64 v15
;; @004e                               v22 = icmp ugt v21, v16
;; @004e                               trapnz v22, user17
;; @004e                               v39 = load.i64 notrap aligned v50+40
;; @004e                               v27 = iconst.i64 20
;; @004e                               v28 = iadd v12, v27  ; v27 = 20
;;                                     v60 = iconst.i64 2
;;                                     v61 = ishl v17, v60  ; v60 = 2
;; @004e                               v32 = iadd v28, v61
;;                                     v63 = ishl v18, v60  ; v60 = 2
;; @004e                               v41 = uadd_overflow_trap v32, v63, user2
;; @004e                               v40 = iadd v11, v39
;; @004e                               v42 = icmp ugt v41, v40
;; @004e                               trapnz v42, user2
;;                                     v58 = iconst.i64 0
;; @004e                               v45 = icmp eq v18, v58  ; v58 = 0
;; @0048                               v5 = iconst.i32 -1
;; @004e                               v30 = iconst.i64 4
;; @004e                               v43 = iadd v32, v63
;; @004e                               brif v45, block3, block2(v32)
;;
;;                                 block2(v46: i64):
;;                                     v65 = iconst.i32 -1
;; @004e                               store user2 little region1 v65, v46  ; v65 = -1
;;                                     v66 = iconst.i64 4
;;                                     v67 = iadd v46, v66  ; v66 = 4
;; @004e                               v49 = icmp eq v67, v43
;; @004e                               brif v49, block3, block2(v67)
;;
;;                                 block3:
;; @0051                               jump block1
;;
;;                                 block1:
;; @0051                               return
;; }
