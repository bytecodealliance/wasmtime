;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc -Ccollector=copying'

(module
  (type $a (array (mut anyref)))

  (func $fill-anything (param $a (ref $a)) (param $i i32) (param $v anyref) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (local.get $v) (local.get $len))
  )

  (func $fill-zero (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.null any) (local.get $len))
  )

  (func $fill-pattern (param $a (ref $a)) (param $i i32) (param $len i32)
    (array.fill $a (local.get $a) (local.get $i) (ref.i31 (i32.const -1)) (local.get $len))
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
;; @0030                               trapz v2, user16
;; @0030                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly v10
;; @0030                               v13 = uextend.i64 v3
;; @0030                               v14 = uextend.i64 v5
;; @0030                               v16 = iadd v13, v14
;; @0030                               v12 = uextend.i64 v11
;; @0030                               v17 = icmp ugt v16, v12
;; @0030                               trapnz v17, user17
;; @0030                               v28 = load.i64 notrap aligned v49+40
;;                                     v45 = iconst.i64 20
;; @0030                               v21 = iadd v8, v45  ; v45 = 20
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v13, v53  ; v53 = 2
;; @0030                               v24 = iadd v21, v54
;;                                     v56 = ishl v14, v53  ; v53 = 2
;; @0030                               v30 = uadd_overflow_trap v24, v56, user2
;; @0030                               v29 = iadd v7, v28
;; @0030                               v31 = icmp ugt v30, v29
;; @0030                               trapnz v31, user2
;;                                     v51 = iconst.i64 0
;; @0030                               v33 = icmp eq v14, v51  ; v51 = 0
;;                                     v44 = iconst.i64 4
;; @0030                               v32 = iadd v24, v56
;; @0030                               brif v33, block3, block2(v24)
;;
;;                                 block2(v34: i64):
;; @0030                               store.i32 user2 little v4, v34
;;                                     v58 = iconst.i64 4
;;                                     v59 = iadd v34, v58  ; v58 = 4
;; @0030                               v36 = icmp eq v59, v32
;; @0030                               brif v36, block3, block2(v59)
;;
;;                                 block3:
;; @0033                               jump block1
;;
;;                                 block1:
;; @0033                               return
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
;; @003e                               trapz v2, user16
;; @003e                               v44 = load.i64 notrap aligned readonly can_move v0+8
;; @003e                               v7 = load.i64 notrap aligned readonly can_move v44+32
;; @003e                               v6 = uextend.i64 v2
;; @003e                               v8 = iadd v7, v6
;; @003e                               v9 = iconst.i64 16
;; @003e                               v10 = iadd v8, v9  ; v9 = 16
;; @003e                               v11 = load.i32 user2 readonly v10
;; @003e                               v13 = uextend.i64 v3
;; @003e                               v14 = uextend.i64 v4
;; @003e                               v16 = iadd v13, v14
;; @003e                               v12 = uextend.i64 v11
;; @003e                               v17 = icmp ugt v16, v12
;; @003e                               trapnz v17, user17
;; @003e                               v28 = load.i64 notrap aligned v44+40
;;                                     v40 = iconst.i64 20
;; @003e                               v21 = iadd v8, v40  ; v40 = 20
;;                                     v48 = iconst.i64 2
;;                                     v49 = ishl v13, v48  ; v48 = 2
;; @003e                               v24 = iadd v21, v49
;;                                     v51 = ishl v14, v48  ; v48 = 2
;; @003e                               v30 = uadd_overflow_trap v24, v51, user2
;; @003e                               v29 = iadd v7, v28
;; @003e                               v31 = icmp ugt v30, v29
;; @003e                               trapnz v31, user2
;; @003a                               v5 = iconst.i32 0
;; @003e                               call fn0(v0, v24, v5, v51)  ; v5 = 0
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32) tail {
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
;; @004e                               v9 = load.i64 notrap aligned readonly can_move v51+32
;; @004e                               v8 = uextend.i64 v2
;; @004e                               v10 = iadd v9, v8
;; @004e                               v11 = iconst.i64 16
;; @004e                               v12 = iadd v10, v11  ; v11 = 16
;; @004e                               v13 = load.i32 user2 readonly v12
;; @004e                               v15 = uextend.i64 v3
;; @004e                               v16 = uextend.i64 v4
;; @004e                               v18 = iadd v15, v16
;; @004e                               v14 = uextend.i64 v13
;; @004e                               v19 = icmp ugt v18, v14
;; @004e                               trapnz v19, user17
;; @004e                               v30 = load.i64 notrap aligned v51+40
;;                                     v47 = iconst.i64 20
;; @004e                               v23 = iadd v10, v47  ; v47 = 20
;;                                     v63 = iconst.i64 2
;;                                     v64 = ishl v15, v63  ; v63 = 2
;; @004e                               v26 = iadd v23, v64
;;                                     v66 = ishl v16, v63  ; v63 = 2
;; @004e                               v32 = uadd_overflow_trap v26, v66, user2
;; @004e                               v31 = iadd v9, v30
;; @004e                               v33 = icmp ugt v32, v31
;; @004e                               trapnz v33, user2
;;                                     v61 = iconst.i64 0
;; @004e                               v35 = icmp eq v16, v61  ; v61 = 0
;; @0048                               v5 = iconst.i32 -1
;;                                     v46 = iconst.i64 4
;; @004e                               v34 = iadd v26, v66
;; @004e                               brif v35, block3, block2(v26)
;;
;;                                 block2(v36: i64):
;;                                     v68 = iconst.i32 -1
;; @004e                               store user2 little v68, v36  ; v68 = -1
;;                                     v69 = iconst.i64 4
;;                                     v70 = iadd v36, v69  ; v69 = 4
;; @004e                               v38 = icmp eq v70, v34
;; @004e                               brif v38, block3, block2(v70)
;;
;;                                 block3:
;; @0051                               jump block1
;;
;;                                 block1:
;; @0051                               return
;; }
