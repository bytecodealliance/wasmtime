;;! target = "x86_64"

(module
  (func $imported (import "env" "f") (param i32) (result i32))
  (func $local (result externref externref funcref funcref)
    global.get 0
    global.get 1
    global.get 2
    global.get 3)

  (global (export "externref-imported") externref (ref.null extern))
  (global (export "externref-local") externref (ref.null extern))
  (global (export "funcref-imported") funcref (ref.func $imported))
  (global (export "funcref-local") funcref (ref.func $local)))

;; function u0:1(i64 vmctx, i64) -> i32, i32, i64, i64 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v94 = iconst.i64 96
;; @008f                               v7 = iadd v0, v94  ; v94 = 96
;; @008f                               v8 = load.i32 notrap aligned v7
;;                                     v95 = stack_addr.i64 ss0
;;                                     store notrap v8, v95
;;                                     v96 = stack_addr.i64 ss0
;;                                     v93 = load.i32 notrap v96
;;                                     v97 = iconst.i32 0
;; @008f                               v9 = icmp eq v93, v97  ; v97 = 0
;; @008f                               brif v9, block5, block2
;;
;;                                 block2:
;; @008f                               v11 = load.i64 notrap aligned readonly v0+56
;; @008f                               v12 = load.i64 notrap aligned v11
;; @008f                               v13 = load.i64 notrap aligned v11+8
;; @008f                               v14 = icmp eq v12, v13
;; @008f                               brif v14, block3, block4
;;
;;                                 block4:
;; @008f                               v16 = load.i64 notrap aligned readonly v0+40
;; @008f                               v18 = load.i64 notrap aligned readonly v0+48
;;                                     v98 = stack_addr.i64 ss0
;;                                     v92 = load.i32 notrap v98
;; @008f                               v19 = uextend.i64 v92
;; @008f                               v20 = iconst.i64 8
;; @008f                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @008f                               v22 = iconst.i64 8
;; @008f                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @008f                               v24 = icmp ule v23, v18
;; @008f                               trapz v24, user1
;; @008f                               v25 = iadd v16, v21
;; @008f                               v26 = load.i64 notrap aligned v25
;;                                     v99 = iconst.i64 1
;; @008f                               v27 = iadd v26, v99  ; v99 = 1
;; @008f                               v29 = load.i64 notrap aligned readonly v0+40
;; @008f                               v31 = load.i64 notrap aligned readonly v0+48
;;                                     v100 = stack_addr.i64 ss0
;;                                     v91 = load.i32 notrap v100
;; @008f                               v32 = uextend.i64 v91
;; @008f                               v33 = iconst.i64 8
;; @008f                               v34 = uadd_overflow_trap v32, v33, user1  ; v33 = 8
;; @008f                               v35 = iconst.i64 8
;; @008f                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @008f                               v37 = icmp ule v36, v31
;; @008f                               trapz v37, user1
;; @008f                               v38 = iadd v29, v34
;; @008f                               store notrap aligned v27, v38
;;                                     v101 = stack_addr.i64 ss0
;;                                     v90 = load.i32 notrap v101
;; @008f                               store notrap aligned v90, v12
;;                                     v102 = iconst.i64 4
;; @008f                               v39 = iadd.i64 v12, v102  ; v102 = 4
;; @008f                               store notrap aligned v39, v11
;; @008f                               jump block5
;;
;;                                 block3 cold:
;;                                     v103 = stack_addr.i64 ss0
;;                                     v89 = load.i32 notrap v103
;; @008f                               v41 = call fn0(v0, v89), stack_map=[i32 @ ss0+0]
;; @008f                               jump block5
;;
;;                                 block5:
;;                                     v104 = iconst.i64 112
;; @0091                               v43 = iadd.i64 v0, v104  ; v104 = 112
;; @0091                               v44 = load.i32 notrap aligned v43
;;                                     v105 = stack_addr.i64 ss1
;;                                     store notrap v44, v105
;;                                     v106 = stack_addr.i64 ss1
;;                                     v88 = load.i32 notrap v106
;;                                     v107 = iconst.i32 0
;; @0091                               v45 = icmp eq v88, v107  ; v107 = 0
;; @0091                               brif v45, block9, block6
;;
;;                                 block6:
;; @0091                               v47 = load.i64 notrap aligned readonly v0+56
;; @0091                               v48 = load.i64 notrap aligned v47
;; @0091                               v49 = load.i64 notrap aligned v47+8
;; @0091                               v50 = icmp eq v48, v49
;; @0091                               brif v50, block7, block8
;;
;;                                 block8:
;; @0091                               v52 = load.i64 notrap aligned readonly v0+40
;; @0091                               v54 = load.i64 notrap aligned readonly v0+48
;;                                     v108 = stack_addr.i64 ss1
;;                                     v87 = load.i32 notrap v108
;; @0091                               v55 = uextend.i64 v87
;; @0091                               v56 = iconst.i64 8
;; @0091                               v57 = uadd_overflow_trap v55, v56, user1  ; v56 = 8
;; @0091                               v58 = iconst.i64 8
;; @0091                               v59 = uadd_overflow_trap v57, v58, user1  ; v58 = 8
;; @0091                               v60 = icmp ule v59, v54
;; @0091                               trapz v60, user1
;; @0091                               v61 = iadd v52, v57
;; @0091                               v62 = load.i64 notrap aligned v61
;;                                     v109 = iconst.i64 1
;; @0091                               v63 = iadd v62, v109  ; v109 = 1
;; @0091                               v65 = load.i64 notrap aligned readonly v0+40
;; @0091                               v67 = load.i64 notrap aligned readonly v0+48
;;                                     v110 = stack_addr.i64 ss1
;;                                     v86 = load.i32 notrap v110
;; @0091                               v68 = uextend.i64 v86
;; @0091                               v69 = iconst.i64 8
;; @0091                               v70 = uadd_overflow_trap v68, v69, user1  ; v69 = 8
;; @0091                               v71 = iconst.i64 8
;; @0091                               v72 = uadd_overflow_trap v70, v71, user1  ; v71 = 8
;; @0091                               v73 = icmp ule v72, v67
;; @0091                               trapz v73, user1
;; @0091                               v74 = iadd v65, v70
;; @0091                               store notrap aligned v63, v74
;;                                     v111 = stack_addr.i64 ss1
;;                                     v85 = load.i32 notrap v111
;; @0091                               store notrap aligned v85, v48
;;                                     v112 = iconst.i64 4
;; @0091                               v75 = iadd.i64 v48, v112  ; v112 = 4
;; @0091                               store notrap aligned v75, v47
;; @0091                               jump block9
;;
;;                                 block7 cold:
;;                                     v113 = stack_addr.i64 ss1
;;                                     v84 = load.i32 notrap v113
;; @0091                               v77 = call fn0(v0, v84), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v79 = load.i64 notrap aligned table v0+128
;; @0095                               v81 = load.i64 notrap aligned table v0+144
;;                                     v114 = stack_addr.i64 ss0
;;                                     v82 = load.i32 notrap v114
;;                                     v115 = stack_addr.i64 ss1
;;                                     v83 = load.i32 notrap v115
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v82, v83, v79, v81
;; }
