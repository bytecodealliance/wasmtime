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
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0030                               trapz v2, user16
;; @0030                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly region0 v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v17 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v18 = icmp ugt v17, v12
;; @0030                               trapnz v18, user17
;; @0030                               v34 = load.i64 notrap aligned v47+40
;; @0030                               v23 = iconst.i64 20
;; @0030                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v13, v51  ; v51 = 2
;; @0030                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 2
;; @0030                               v36 = uadd_overflow_trap v28, v54, user2
;; @0030                               v35 = iadd v7, v34
;; @0030                               v37 = icmp ugt v36, v35
;; @0030                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @0030                               v40 = icmp eq v14, v49  ; v49 = 0
;; @0030                               v26 = iconst.i64 4
;; @0030                               v38 = iadd v28, v54
;; @0030                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;; @0030                               store.i32 user2 little region0 v4, v41
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v41, v56  ; v56 = 4
;; @0030                               v44 = icmp eq v57, v38
;; @0030                               brif v44, block3, block2(v57)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003e                               trapz v2, user16
;; @003e                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @003e                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @003e                               v6 = uextend.i64 v2
;; @003e                               v8 = iadd v7, v6
;; @003e                               v9 = iconst.i64 16
;; @003e                               v10 = iadd v8, v9  ; v9 = 16
;; @003e                               v11 = load.i32 user2 readonly region0 v10
;; @003e                               v13 = uextend.i64 v3
;; @003e                               v14 = uextend.i64 v4
;; @003e                               v17 = iadd v13, v14
;; @003e                               v12 = uextend.i64 v11
;; @003e                               v18 = icmp ugt v17, v12
;; @003e                               trapnz v18, user17
;; @003e                               v34 = load.i64 notrap aligned v47+40
;; @003e                               v23 = iconst.i64 20
;; @003e                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v13, v51  ; v51 = 2
;; @003e                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 2
;; @003e                               v36 = uadd_overflow_trap v28, v54, user2
;; @003e                               v35 = iadd v7, v34
;; @003e                               v37 = icmp ugt v36, v35
;; @003e                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @003e                               v40 = icmp eq v14, v49  ; v49 = 0
;; @003a                               v5 = iconst.i32 0
;; @003e                               v26 = iconst.i64 4
;; @003e                               v38 = iadd v28, v54
;; @003e                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;;                                     v56 = iconst.i32 0
;; @003e                               store user2 little region0 v56, v41  ; v56 = 0
;;                                     v57 = iconst.i64 4
;;                                     v58 = iadd v41, v57  ; v57 = 4
;; @003e                               v44 = icmp eq v58, v38
;; @003e                               brif v44, block3, block2(v58)
;;
;;                                 block3:
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @004e                               trapz v2, user16
;; @004e                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @004e                               v11 = load.i64 notrap aligned readonly can_move v51+32
;; @004e                               v10 = uextend.i64 v2
;; @004e                               v12 = iadd v11, v10
;; @004e                               v13 = iconst.i64 16
;; @004e                               v14 = iadd v12, v13  ; v13 = 16
;; @004e                               v15 = load.i32 user2 readonly region0 v14
;; @004e                               v17 = uextend.i64 v3
;; @004e                               v18 = uextend.i64 v4
;; @004e                               v21 = iadd v17, v18
;; @004e                               v16 = uextend.i64 v15
;; @004e                               v22 = icmp ugt v21, v16
;; @004e                               trapnz v22, user17
;; @004e                               v38 = load.i64 notrap aligned v51+40
;; @004e                               v27 = iconst.i64 20
;; @004e                               v28 = iadd v12, v27  ; v27 = 20
;;                                     v61 = iconst.i64 2
;;                                     v62 = ishl v17, v61  ; v61 = 2
;; @004e                               v32 = iadd v28, v62
;;                                     v64 = ishl v18, v61  ; v61 = 2
;; @004e                               v40 = uadd_overflow_trap v32, v64, user2
;; @004e                               v39 = iadd v11, v38
;; @004e                               v41 = icmp ugt v40, v39
;; @004e                               trapnz v41, user2
;;                                     v59 = iconst.i64 0
;; @004e                               v44 = icmp eq v18, v59  ; v59 = 0
;; @0048                               v5 = iconst.i32 -1
;; @004e                               v30 = iconst.i64 4
;; @004e                               v42 = iadd v32, v64
;; @004e                               brif v44, block3, block2(v32)
;;
;;                                 block2(v45: i64):
;;                                     v66 = iconst.i32 -1
;; @004e                               store user2 little region0 v66, v45  ; v66 = -1
;;                                     v67 = iconst.i64 4
;;                                     v68 = iadd v45, v67  ; v67 = 4
;; @004e                               v48 = icmp eq v68, v42
;; @004e                               brif v48, block3, block2(v68)
;;
;;                                 block3:
;; @0051                               jump block1
;;
;;                                 block1:
;; @0051                               return
;; }
