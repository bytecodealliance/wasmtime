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
;; @002f                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @002f                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @002f                               v6 = uextend.i64 v2
;; @002f                               v8 = iadd v7, v6
;; @002f                               v9 = iconst.i64 16
;; @002f                               v10 = iadd v8, v9  ; v9 = 16
;; @002f                               v11 = load.i32 user2 readonly v10
;; @002f                               v12 = uadd_overflow_trap v3, v5, user17
;; @002f                               v13 = icmp ugt v12, v11
;; @002f                               trapnz v13, user17
;; @002f                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v15, v43  ; v43 = 2
;;                                     v40 = iconst.i64 32
;; @002f                               v17 = ushr v44, v40  ; v40 = 32
;; @002f                               trapnz v17, user2
;;                                     v53 = iconst.i32 2
;;                                     v54 = ishl v11, v53  ; v53 = 2
;; @002f                               v19 = iconst.i32 20
;; @002f                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 20
;; @002f                               v24 = uadd_overflow_trap v2, v20, user2
;; @002f                               v25 = uextend.i64 v24
;; @002f                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 2
;;                                     v62 = iadd v60, v19  ; v19 = 20
;; @002f                               v28 = isub v20, v62
;; @002f                               v29 = uextend.i64 v28
;; @002f                               v30 = isub v27, v29
;;                                     v64 = ishl v5, v53  ; v53 = 2
;; @002f                               v32 = uextend.i64 v64
;; @002f                               v33 = iadd v30, v32
;; @002f                               v14 = iconst.i64 4
;; @002f                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @002f                               v36 = icmp eq v35, v33
;; @002f                               brif v36, block4, block3
;;
;;                                 block3:
;; @002f                               store.i32 user2 little v4, v35
;;                                     v66 = iconst.i64 4
;;                                     v67 = iadd.i64 v35, v66  ; v66 = 4
;; @002f                               jump block2(v67)
;;
;;                                 block4:
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
;;     fn0 = colocated u805306368:5 sig0
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
;; @003d                               v11 = load.i32 user2 readonly v10
;; @003d                               v12 = uadd_overflow_trap v3, v4, user17
;; @003d                               v13 = icmp ugt v12, v11
;; @003d                               trapnz v13, user17
;; @003d                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v15, v51  ; v51 = 2
;;                                     v48 = iconst.i64 32
;; @003d                               v17 = ushr v52, v48  ; v48 = 32
;; @003d                               trapnz v17, user2
;;                                     v61 = iconst.i32 2
;;                                     v62 = ishl v11, v61  ; v61 = 2
;; @003d                               v19 = iconst.i32 20
;; @003d                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 20
;; @003d                               v24 = uadd_overflow_trap v2, v20, user2
;; @003d                               v37 = load.i64 notrap aligned v49+40
;; @003d                               v25 = uextend.i64 v24
;; @003d                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 2
;;                                     v70 = iadd v68, v19  ; v19 = 20
;; @003d                               v28 = isub v20, v70
;; @003d                               v29 = uextend.i64 v28
;; @003d                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 2
;; @003d                               v32 = uextend.i64 v72
;; @003d                               v33 = iadd v30, v32
;; @003d                               v38 = iadd v7, v37
;; @003d                               v39 = icmp ugt v33, v38
;; @003d                               trapnz v39, user2
;; @0039                               v5 = iconst.i32 0
;; @003d                               call fn0(v0, v30, v5, v32)  ; v5 = 0
;; @0040                               jump block1
;;
;;                                 block1:
;; @0040                               return
;; }
