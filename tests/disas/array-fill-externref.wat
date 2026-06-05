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
;; @002f                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @002f                               v7 = load.i64 notrap aligned readonly can_move v49+32
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
;; @002f                               v32 = load.i64 notrap aligned v49+40
;; @002f                               v22 = iconst.i64 20
;; @002f                               v23 = iadd v8, v22  ; v22 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @002f                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @002f                               v34 = uadd_overflow_trap v27, v56, user2
;; @002f                               v33 = iadd v7, v32
;; @002f                               v35 = icmp ugt v34, v33
;; @002f                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @002f                               v38 = icmp eq v14, v51  ; v51 = 0
;; @002f                               v25 = iconst.i64 4
;; @002f                               v36 = iadd v27, v56
;; @002f                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;; @002f                               store.i32 user2 little region0 v4, v39
;;                                     v58 = iconst.i64 4
;;                                     v59 = iadd v39, v58  ; v58 = 4
;; @002f                               v42 = icmp eq v59, v36
;; @002f                               brif v42, block3, block2(v59)
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
;; @003d                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @003d                               v7 = load.i64 notrap aligned readonly can_move v49+32
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
;; @003d                               v32 = load.i64 notrap aligned v49+40
;; @003d                               v22 = iconst.i64 20
;; @003d                               v23 = iadd v8, v22  ; v22 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @003d                               v27 = iadd v23, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @003d                               v34 = uadd_overflow_trap v27, v56, user2
;; @003d                               v33 = iadd v7, v32
;; @003d                               v35 = icmp ugt v34, v33
;; @003d                               trapnz v35, user2
;;                                     v51 = iconst.i64 0
;; @003d                               v38 = icmp eq v14, v51  ; v51 = 0
;; @0039                               v5 = iconst.i32 0
;; @003d                               v25 = iconst.i64 4
;; @003d                               v36 = iadd v27, v56
;; @003d                               brif v38, block3, block2(v27)
;;
;;                                 block2(v39: i64):
;;                                     v58 = iconst.i32 0
;; @003d                               store user2 little region0 v58, v39  ; v58 = 0
;;                                     v59 = iconst.i64 4
;;                                     v60 = iadd v39, v59  ; v59 = 4
;; @003d                               v42 = icmp eq v60, v36
;; @003d                               brif v42, block3, block2(v60)
;;
;;                                 block3:
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
