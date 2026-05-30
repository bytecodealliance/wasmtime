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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
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
;; @0025                               v6 = load.i32 notrap aligned v0+56
;; @0025                               v8 = uextend.i64 v2
;; @0025                               v9 = uextend.i64 v3
;; @0025                               v12 = iadd v8, v9
;; @0025                               v7 = uextend.i64 v6
;; @0025                               v13 = icmp ugt v12, v7
;; @0025                               trapnz v13, heap_oob
;; @0025                               v15 = load.i64 notrap aligned v0+48
;;                                     v128 = iconst.i64 32
;; @0025                               v22 = ushr v9, v128  ; v128 = 32
;; @0025                               trapnz v22, user18
;; @0025                               v18 = iconst.i32 20
;; @0025                               v24 = uadd_overflow_trap v18, v3, user18  ; v18 = 20
;; @0025                               v26 = load.i64 notrap aligned readonly can_move v0+32
;; @0025                               v27 = load.i32 notrap aligned v26
;; @0025                               v28 = load.i32 notrap aligned v26+4
;; @0025                               v34 = uextend.i64 v27
;; @0025                               v29 = uextend.i64 v24
;; @0025                               v30 = iconst.i64 15
;; @0025                               v32 = iadd v29, v30  ; v30 = 15
;; @0025                               v31 = iconst.i64 -16
;; @0025                               v33 = band v32, v31  ; v31 = -16
;; @0025                               v35 = iadd v34, v33
;; @0025                               v36 = uextend.i64 v28
;; @0025                               v37 = icmp ule v35, v36
;; @0025                               brif v37, block2, block3
;;
;;                                 block2:
;;                                     v136 = iconst.i32 15
;;                                     v137 = iadd.i32 v24, v136  ; v136 = 15
;;                                     v140 = iconst.i32 -16
;;                                     v141 = band v137, v140  ; v140 = -16
;;                                     v143 = iadd.i32 v27, v141
;; @0025                               store notrap aligned region0 v143, v26
;;                                     v157 = iconst.i32 -1476395002
;;                                     v158 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v159 = load.i64 notrap aligned readonly can_move v158+32
;; @0025                               v51 = iadd v159, v34
;; @0025                               store notrap aligned v157, v51  ; v157 = -1476395002
;;                                     v160 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v161 = load.i32 notrap aligned readonly can_move v160
;; @0025                               store notrap aligned v161, v51+4
;;                                     v162 = band.i64 v32, v31  ; v31 = -16
;; @0025                               istore32 notrap aligned v162, v51+8
;; @0025                               jump block4(v27, v51)
;;
;;                                 block3 cold:
;; @0025                               v39 = iconst.i32 -1476395002
;; @0025                               v41 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v42 = load.i32 notrap aligned readonly can_move v41
;; @0025                               v43 = iconst.i32 16
;; @0025                               v44 = call fn0(v0, v39, v42, v24, v43)  ; v39 = -1476395002, v43 = 16
;; @0025                               v124 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v45 = load.i64 notrap aligned readonly can_move v124+32
;; @0025                               v46 = uextend.i64 v44
;; @0025                               v47 = iadd v45, v46
;; @0025                               jump block4(v44, v47)
;;
;;                                 block4(v56: i32, v57: i64):
;;                                     v123 = stack_addr.i64 ss0
;;                                     store notrap v56, v123
;; @0025                               v58 = iconst.i64 16
;; @0025                               v59 = iadd v57, v58  ; v58 = 16
;; @0025                               store.i32 user2 region1 v3, v59
;; @0025                               trapz v56, user16
;;                                     v163 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v164 = load.i64 notrap aligned readonly can_move v163+32
;; @0025                               v61 = uextend.i64 v56
;; @0025                               v63 = iadd v164, v61
;; @0025                               v65 = iadd v63, v58  ; v58 = 16
;; @0025                               v66 = load.i32 user2 readonly region1 v65
;; @0025                               v67 = uextend.i64 v66
;; @0025                               v73 = icmp.i64 ugt v9, v67
;; @0025                               trapnz v73, user17
;; @0025                               v84 = load.i32 notrap aligned v0+56
;; @0025                               v85 = uextend.i64 v84
;; @0025                               v91 = icmp.i64 ugt v12, v85
;; @0025                               trapnz v91, heap_oob
;; @0025                               v93 = load.i64 notrap aligned v0+48
;; @0025                               v102 = load.i64 notrap aligned v163+40
;; @0025                               v77 = iconst.i64 20
;; @0025                               v78 = iadd v63, v77  ; v77 = 20
;; @0025                               v104 = uadd_overflow_trap v78, v9, user2
;; @0025                               v103 = iadd v164, v102
;; @0025                               v105 = icmp ugt v104, v103
;; @0025                               trapnz v105, user2
;; @0025                               v95 = iadd v93, v8
;; @0025                               call fn1(v0, v78, v95, v9), stack_map=[i32 @ ss0+0]
;;                                     v107 = load.i32 notrap v123
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v107
;; }
