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
;; @0030                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0030                               v7 = load.i64 notrap aligned readonly can_move v41+32
;; @0030                               v6 = uextend.i64 v2
;; @0030                               v8 = iadd v7, v6
;; @0030                               v9 = iconst.i64 16
;; @0030                               v10 = iadd v8, v9  ; v9 = 16
;; @0030                               v11 = load.i32 user2 readonly v10
;; @0030                               v12 = uadd_overflow_trap v3, v5, user17
;; @0030                               v13 = icmp ugt v12, v11
;; @0030                               trapnz v13, user17
;; @0030                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 2
;;                                     v44 = ishl v15, v43  ; v43 = 2
;;                                     v40 = iconst.i64 32
;; @0030                               v17 = ushr v44, v40  ; v40 = 32
;; @0030                               trapnz v17, user2
;;                                     v53 = iconst.i32 2
;;                                     v54 = ishl v11, v53  ; v53 = 2
;; @0030                               v19 = iconst.i32 20
;; @0030                               v20 = uadd_overflow_trap v54, v19, user2  ; v19 = 20
;; @0030                               v24 = uadd_overflow_trap v2, v20, user2
;; @0030                               v25 = uextend.i64 v24
;; @0030                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 2
;;                                     v62 = iadd v60, v19  ; v19 = 20
;; @0030                               v28 = isub v20, v62
;; @0030                               v29 = uextend.i64 v28
;; @0030                               v30 = isub v27, v29
;;                                     v64 = ishl v5, v53  ; v53 = 2
;; @0030                               v32 = uextend.i64 v64
;; @0030                               v33 = iadd v30, v32
;; @0030                               v14 = iconst.i64 4
;; @0030                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @0030                               v36 = icmp eq v35, v33
;; @0030                               brif v36, block4, block3
;;
;;                                 block3:
;; @0030                               store.i32 user2 little v4, v35
;;                                     v66 = iconst.i64 4
;;                                     v67 = iadd.i64 v35, v66  ; v66 = 4
;; @0030                               jump block2(v67)
;;
;;                                 block4:
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
;;     fn0 = colocated u805306368:5 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003e                               trapz v2, user16
;; @003e                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @003e                               v7 = load.i64 notrap aligned readonly can_move v49+32
;; @003e                               v6 = uextend.i64 v2
;; @003e                               v8 = iadd v7, v6
;; @003e                               v9 = iconst.i64 16
;; @003e                               v10 = iadd v8, v9  ; v9 = 16
;; @003e                               v11 = load.i32 user2 readonly v10
;; @003e                               v12 = uadd_overflow_trap v3, v4, user17
;; @003e                               v13 = icmp ugt v12, v11
;; @003e                               trapnz v13, user17
;; @003e                               v15 = uextend.i64 v11
;;                                     v51 = iconst.i64 2
;;                                     v52 = ishl v15, v51  ; v51 = 2
;;                                     v48 = iconst.i64 32
;; @003e                               v17 = ushr v52, v48  ; v48 = 32
;; @003e                               trapnz v17, user2
;;                                     v61 = iconst.i32 2
;;                                     v62 = ishl v11, v61  ; v61 = 2
;; @003e                               v19 = iconst.i32 20
;; @003e                               v20 = uadd_overflow_trap v62, v19, user2  ; v19 = 20
;; @003e                               v24 = uadd_overflow_trap v2, v20, user2
;; @003e                               v37 = load.i64 notrap aligned v49+40
;; @003e                               v25 = uextend.i64 v24
;; @003e                               v27 = iadd v7, v25
;;                                     v68 = ishl v3, v61  ; v61 = 2
;;                                     v70 = iadd v68, v19  ; v19 = 20
;; @003e                               v28 = isub v20, v70
;; @003e                               v29 = uextend.i64 v28
;; @003e                               v30 = isub v27, v29
;;                                     v72 = ishl v4, v61  ; v61 = 2
;; @003e                               v32 = uextend.i64 v72
;; @003e                               v33 = iadd v30, v32
;; @003e                               v38 = iadd v7, v37
;; @003e                               v39 = icmp ugt v33, v38
;; @003e                               trapnz v39, user2
;; @003a                               v5 = iconst.i32 0
;; @003e                               call fn0(v0, v30, v5, v32)  ; v5 = 0
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
;; @004e                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @004e                               v9 = load.i64 notrap aligned readonly can_move v43+32
;; @004e                               v8 = uextend.i64 v2
;; @004e                               v10 = iadd v9, v8
;; @004e                               v11 = iconst.i64 16
;; @004e                               v12 = iadd v10, v11  ; v11 = 16
;; @004e                               v13 = load.i32 user2 readonly v12
;; @004e                               v14 = uadd_overflow_trap v3, v4, user17
;; @004e                               v15 = icmp ugt v14, v13
;; @004e                               trapnz v15, user17
;; @004e                               v17 = uextend.i64 v13
;;                                     v53 = iconst.i64 2
;;                                     v54 = ishl v17, v53  ; v53 = 2
;;                                     v42 = iconst.i64 32
;; @004e                               v19 = ushr v54, v42  ; v42 = 32
;; @004e                               trapnz v19, user2
;;                                     v63 = iconst.i32 2
;;                                     v64 = ishl v13, v63  ; v63 = 2
;; @004e                               v21 = iconst.i32 20
;; @004e                               v22 = uadd_overflow_trap v64, v21, user2  ; v21 = 20
;; @004e                               v26 = uadd_overflow_trap v2, v22, user2
;; @004e                               v27 = uextend.i64 v26
;; @004e                               v29 = iadd v9, v27
;;                                     v70 = ishl v3, v63  ; v63 = 2
;;                                     v72 = iadd v70, v21  ; v21 = 20
;; @004e                               v30 = isub v22, v72
;; @004e                               v31 = uextend.i64 v30
;; @004e                               v32 = isub v29, v31
;;                                     v74 = ishl v4, v63  ; v63 = 2
;; @004e                               v34 = uextend.i64 v74
;; @004e                               v35 = iadd v32, v34
;; @0048                               v5 = iconst.i32 -1
;; @004e                               v16 = iconst.i64 4
;; @004e                               jump block2(v32)
;;
;;                                 block2(v37: i64):
;; @004e                               v38 = icmp eq v37, v35
;; @004e                               brif v38, block4, block3
;;
;;                                 block3:
;;                                     v76 = iconst.i32 -1
;; @004e                               store user2 little v76, v37  ; v76 = -1
;;                                     v77 = iconst.i64 4
;;                                     v78 = iadd.i64 v37, v77  ; v77 = 4
;; @004e                               jump block2(v78)
;;
;;                                 block4:
;; @0051                               jump block1
;;
;;                                 block1:
;; @0051                               return
;; }
