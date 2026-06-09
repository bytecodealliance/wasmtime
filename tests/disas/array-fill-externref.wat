;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut externref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v externref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null extern) (local.get $len))
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
;; @002f                               trapz v2, user16
;; @002f                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @002f                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @002f                               v6 = uextend.i64 v2
;; @002f                               v8 = iadd v7, v6
;; @002f                               v9 = iconst.i64 16
;; @002f                               v10 = iadd v8, v9  ; v9 = 16
;; @002f                               v11 = load.i32 user2 readonly region0 v10
;; @002f                               v13 = uextend.i64 v3
;; @002f                               v14 = uextend.i64 v5
;; @002f                               v17 = iadd v13, v14
;; @002f                               v12 = uextend.i64 v11
;; @002f                               v18 = icmp ugt v17, v12
;; @002f                               trapnz v18, user17
;; @002f                               v34 = load.i64 notrap aligned v47+40
;; @002f                               v23 = iconst.i64 20
;; @002f                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v13, v51  ; v51 = 2
;; @002f                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 2
;; @002f                               v36 = uadd_overflow_trap v28, v54, user2
;; @002f                               v35 = iadd v7, v34
;; @002f                               v37 = icmp ugt v36, v35
;; @002f                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @002f                               v40 = icmp eq v14, v49  ; v49 = 0
;; @002f                               v26 = iconst.i64 4
;; @002f                               v38 = iadd v28, v54
;; @002f                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;; @002f                               store.i32 user2 little region0 v4, v41
;;                                     v56 = iconst.i64 4
;;                                     v57 = iadd v41, v56  ; v56 = 4
;; @002f                               v44 = icmp eq v57, v38
;; @002f                               brif v44, block3, block2(v57)
;;
;;                                 block3:
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return
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
;; @003d                               trapz v2, user16
;; @003d                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @003d                               v7 = load.i64 notrap aligned readonly can_move v47+32
;; @003d                               v6 = uextend.i64 v2
;; @003d                               v8 = iadd v7, v6
;; @003d                               v9 = iconst.i64 16
;; @003d                               v10 = iadd v8, v9  ; v9 = 16
;; @003d                               v11 = load.i32 user2 readonly region0 v10
;; @003d                               v13 = uextend.i64 v3
;; @003d                               v14 = uextend.i64 v4
;; @003d                               v17 = iadd v13, v14
;; @003d                               v12 = uextend.i64 v11
;; @003d                               v18 = icmp ugt v17, v12
;; @003d                               trapnz v18, user17
;; @003d                               v34 = load.i64 notrap aligned v47+40
;; @003d                               v23 = iconst.i64 20
;; @003d                               v24 = iadd v8, v23  ; v23 = 20
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v13, v51  ; v51 = 2
;; @003d                               v28 = iadd v24, v52
;;                                     v54 = ishl v14, v51  ; v51 = 2
;; @003d                               v36 = uadd_overflow_trap v28, v54, user2
;; @003d                               v35 = iadd v7, v34
;; @003d                               v37 = icmp ugt v36, v35
;; @003d                               trapnz v37, user2
;;                                     v49 = iconst.i64 0
;; @003d                               v40 = icmp eq v14, v49  ; v49 = 0
;; @0039                               v5 = iconst.i32 0
;; @003d                               v26 = iconst.i64 4
;; @003d                               v38 = iadd v28, v54
;; @003d                               brif v40, block3, block2(v28)
;;
;;                                 block2(v41: i64):
;;                                     v56 = iconst.i32 0
;; @003d                               store user2 little region0 v56, v41  ; v56 = 0
;;                                     v57 = iconst.i64 4
;;                                     v58 = iadd v41, v57  ; v57 = 4
;; @003d                               v44 = icmp eq v58, v38
;; @003d                               brif v44, block3, block2(v58)
;;
;;                                 block3:
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
