;;! target = "x86_64"
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (data $passive "this is a passive data segment")
  (type $a (array i8))

  (func $a (param i32 i32) (result (ref $a))
    local.get 0
    local.get 1
    array.new_data $a $passive)
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:6 sig0
;;     fn1 = colocated u805306368:26 sig1
;;     fn2 = colocated u805306368:4 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0025                               v5 = uextend.i64 v3
;;                                     v72 = iconst.i64 32
;; @0025                               v7 = ushr v5, v72  ; v72 = 32
;; @0025                               trapnz v7, heap_oob
;; @0025                               v10 = uload32 notrap aligned v0+48
;; @0025                               v11 = uextend.i64 v2
;; @0025                               v13 = iadd v11, v5
;; @0025                               v14 = icmp ugt v13, v10
;; @0025                               trapnz v14, heap_oob
;; @0025                               v15 = iconst.i32 0
;; @0025                               v16 = call fn0(v0, v15, v2, v3)  ; v15 = 0
;; @0025                               trapnz v7, user18
;; @0025                               v17 = iconst.i32 28
;; @0025                               v22 = uadd_overflow_trap v17, v3, user18  ; v17 = 28
;; @0025                               v24 = iconst.i32 -1476395008
;; @0025                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v27 = load.i32 notrap aligned readonly can_move v26
;; @0025                               v28 = iconst.i32 8
;; @0025                               v29 = call fn1(v0, v24, v27, v22, v28)  ; v24 = -1476395008, v28 = 8
;;                                     v69 = stack_addr.i64 ss0
;;                                     store notrap v29, v69
;; @0025                               v67 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v30 = load.i64 notrap aligned readonly can_move v67+32
;; @0025                               v31 = uextend.i64 v29
;; @0025                               v32 = iadd v30, v31
;;                                     v65 = iconst.i64 24
;; @0025                               v33 = iadd v32, v65  ; v65 = 24
;; @0025                               store user2 v3, v33
;; @0025                               v41 = ushr v5, v72  ; v72 = 32
;; @0025                               trapnz v41, user2
;; @0025                               v44 = uadd_overflow_trap v3, v17, user2  ; v17 = 28
;;                                     v58 = load.i32 notrap v69
;; @0025                               v48 = uadd_overflow_trap v58, v44, user2
;; @0025                               v49 = uextend.i64 v48
;; @0025                               v51 = iadd v30, v49
;; @0025                               v52 = isub v44, v17  ; v17 = 28
;; @0025                               v53 = uextend.i64 v52
;; @0025                               v54 = isub v51, v53
;; @0025                               call fn2(v0, v54, v16, v5), stack_map=[i32 @ ss0+0]
;;                                     v57 = load.i32 notrap v69
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v57
;; }

;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0025                               v6 = uload32 notrap aligned v0+56
;; @0025                               v7 = uextend.i64 v2
;; @0025                               v8 = uextend.i64 v3
;; @0025                               v10 = iadd v7, v8
;; @0025                               v11 = icmp ugt v10, v6
;; @0025                               trapnz v11, heap_oob
;; @0025                               v13 = load.i64 notrap aligned v0+48
;;                                     v124 = iconst.i64 32
;; @0025                               v19 = ushr v8, v124  ; v124 = 32
;; @0025                               trapnz v19, user18
;; @0025                               v16 = iconst.i32 20
;; @0025                               v21 = uadd_overflow_trap v16, v3, user18  ; v16 = 20
;; @0025                               v23 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v24 = load.i32 notrap aligned v23
;; @0025                               v25 = load.i32 notrap aligned v23+4
;; @0025                               v31 = uextend.i64 v24
;; @0025                               v26 = uextend.i64 v21
;; @0025                               v27 = iconst.i64 15
;; @0025                               v29 = iadd v26, v27  ; v27 = 15
;; @0025                               v28 = iconst.i64 -16
;; @0025                               v30 = band v29, v28  ; v28 = -16
;; @0025                               v32 = iadd v31, v30
;; @0025                               v33 = uextend.i64 v25
;; @0025                               v34 = icmp ule v32, v33
;; @0025                               brif v34, block2, block3
;;
;;                                 block2:
;;                                     v134 = iconst.i32 15
;;                                     v135 = iadd.i32 v21, v134  ; v134 = 15
;;                                     v138 = iconst.i32 -16
;;                                     v139 = band v135, v138  ; v138 = -16
;;                                     v141 = iadd.i32 v24, v139
;; @0025                               store notrap aligned vmctx v141, v23
;;                                     v155 = iconst.i32 -1476395008
;;                                     v156 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v157 = load.i64 notrap aligned readonly can_move v156+32
;; @0025                               v48 = iadd v157, v31
;; @0025                               store notrap aligned v155, v48  ; v155 = -1476395008
;;                                     v158 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v159 = load.i32 notrap aligned readonly can_move v158
;; @0025                               store notrap aligned v159, v48+4
;;                                     v160 = band.i64 v29, v28  ; v28 = -16
;; @0025                               istore32 notrap aligned v160, v48+8
;; @0025                               jump block4(v24, v48)
;;
;;                                 block3 cold:
;; @0025                               v36 = iconst.i32 -1476395008
;; @0025                               v38 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               v40 = iconst.i32 16
;; @0025                               v41 = call fn0(v0, v36, v39, v21, v40)  ; v36 = -1476395008, v40 = 16
;; @0025                               v120 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v42 = load.i64 notrap aligned readonly can_move v120+32
;; @0025                               v43 = uextend.i64 v41
;; @0025                               v44 = iadd v42, v43
;; @0025                               jump block4(v41, v44)
;;
;;                                 block4(v53: i32, v54: i64):
;;                                     v119 = stack_addr.i64 ss0
;;                                     store notrap v53, v119
;;                                     v118 = iconst.i64 16
;; @0025                               v55 = iadd v54, v118  ; v118 = 16
;; @0025                               store.i32 user2 v3, v55
;;                                     v99 = load.i32 notrap v119
;; @0025                               trapz v99, user16
;;                                     v161 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v162 = load.i64 notrap aligned readonly can_move v161+32
;; @0025                               v57 = uextend.i64 v99
;; @0025                               v59 = iadd v162, v57
;; @0025                               v61 = iadd v59, v118  ; v118 = 16
;; @0025                               v62 = load.i32 user2 readonly v61
;; @0025                               v63 = uextend.i64 v62
;; @0025                               v68 = icmp.i64 ugt v8, v63
;; @0025                               trapnz v68, user17
;; @0025                               v77 = uload32.i64 notrap aligned v0+56
;; @0025                               v82 = icmp.i64 ugt v10, v77
;; @0025                               trapnz v82, heap_oob
;; @0025                               v84 = load.i64 notrap aligned v0+48
;; @0025                               v91 = load.i64 notrap aligned v161+40
;;                                     v109 = iconst.i64 20
;; @0025                               v72 = iadd v59, v109  ; v109 = 20
;; @0025                               v93 = uadd_overflow_trap v72, v8, user2
;; @0025                               v92 = iadd v162, v91
;; @0025                               v94 = icmp ugt v93, v92
;; @0025                               trapnz v94, user2
;; @0025                               v86 = iadd v84, v7
;; @0025                               call fn1(v0, v72, v86, v8), stack_map=[i32 @ ss0+0]
;;                                     v96 = load.i32 notrap v119
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v96
;; }
