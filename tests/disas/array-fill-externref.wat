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
;; @002f                               v11 = load.i32 user2 readonly v10
;; @002f                               v13 = uextend.i64 v3
;; @002f                               v14 = uextend.i64 v5
;; @002f                               v16 = iadd v13, v14
;; @002f                               v12 = uextend.i64 v11
;; @002f                               v17 = icmp ugt v16, v12
;; @002f                               trapnz v17, user17
;; @002f                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @002f                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @002f                               v24 = iadd v21, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @002f                               v30 = uadd_overflow_trap v24, v56, user2
;; @002f                               v29 = iadd v7, v28
;; @002f                               v31 = icmp ugt v30, v29
;; @002f                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @002f                               v33 = icmp eq v14, v51  ; v51 = 0
;;                                     v44 = iconst.i64 4
;; @002f                               v32 = iadd v24, v56
;; @002f                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;; @002f                               store.i32 user2 little v4, v34
;;                                     v58 = iconst.i64 4
;;                                     v59 = iadd v34, v58  ; v58 = 4
;; @002f                               v36 = icmp eq v59, v32
;; @002f                               brif v36, block3, block2(v59)
;;
;;                                 block3:
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return
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
;; @003d                               trapz v2, user16
;; @003d                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @003d                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @003d                               v6 = uextend.i64 v2
;; @003d                               v8 = iadd v7, v6
;; @003d                               v9 = iconst.i64 16
;; @003d                               v10 = iadd v8, v9  ; v9 = 16
;; @003d                               v11 = load.i32 user2 readonly v10
;; @003d                               v13 = uextend.i64 v3
;; @003d                               v14 = uextend.i64 v4
;; @003d                               v16 = iadd v13, v14
;; @003d                               v12 = uextend.i64 v11
;; @003d                               v17 = icmp ugt v16, v12
;; @003d                               trapnz v17, user17
;; @003d                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @003d                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v48 = iconst.i64 2
;;                                     v49 = ishl v13, v48  ; v48 = 2
;; @003d                               v24 = iadd v21, v49
;;                                     v51 = ishl v14, v48  ; v48 = 2
;; @003d                               v30 = uadd_overflow_trap v24, v51, user2
;; @003d                               v29 = iadd v7, v28
;; @003d                               v31 = icmp ugt v30, v29
;; @003d                               trapnz v31, user2
;; @0039                               v5 = iconst.i32 0
;; @003d                               call fn0(v0, v24, v5, v51)  ; v5 = 0
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
