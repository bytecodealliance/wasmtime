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
;;                                     v102 = iconst.i64 96
;; @008f                               v7 = iadd v0, v102  ; v102 = 96
;; @008f                               v8 = load.i32 notrap aligned readonly can_move v7
;;                                     v103 = stack_addr.i64 ss0
;;                                     store notrap v8, v103
;;                                     v104 = stack_addr.i64 ss0
;;                                     v101 = load.i32 notrap v104
;;                                     v105 = iconst.i32 1
;; @008f                               v9 = band v101, v105  ; v105 = 1
;;                                     v106 = stack_addr.i64 ss0
;;                                     v100 = load.i32 notrap v106
;;                                     v107 = iconst.i32 0
;; @008f                               v10 = icmp eq v100, v107  ; v107 = 0
;; @008f                               v11 = uextend.i32 v10
;; @008f                               v12 = bor v9, v11
;; @008f                               brif v12, block5, block2
;;
;;                                 block2:
;; @008f                               v14 = load.i64 notrap aligned readonly v0+56
;; @008f                               v15 = load.i64 notrap aligned v14
;; @008f                               v16 = load.i64 notrap aligned v14+8
;; @008f                               v17 = icmp eq v15, v16
;; @008f                               brif v17, block3, block4
;;
;;                                 block4:
;; @008f                               v19 = load.i64 notrap aligned readonly can_move v0+40
;; @008f                               v21 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v108 = stack_addr.i64 ss0
;;                                     v99 = load.i32 notrap v108
;; @008f                               v22 = uextend.i64 v99
;; @008f                               v23 = iconst.i64 8
;; @008f                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @008f                               v25 = iconst.i64 8
;; @008f                               v26 = uadd_overflow_trap v24, v25, user1  ; v25 = 8
;; @008f                               v27 = icmp ule v26, v21
;; @008f                               trapz v27, user1
;; @008f                               v28 = iadd v19, v24
;; @008f                               v29 = load.i64 notrap aligned v28
;;                                     v109 = iconst.i64 1
;; @008f                               v30 = iadd v29, v109  ; v109 = 1
;; @008f                               v32 = load.i64 notrap aligned readonly can_move v0+40
;; @008f                               v34 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v110 = stack_addr.i64 ss0
;;                                     v98 = load.i32 notrap v110
;; @008f                               v35 = uextend.i64 v98
;; @008f                               v36 = iconst.i64 8
;; @008f                               v37 = uadd_overflow_trap v35, v36, user1  ; v36 = 8
;; @008f                               v38 = iconst.i64 8
;; @008f                               v39 = uadd_overflow_trap v37, v38, user1  ; v38 = 8
;; @008f                               v40 = icmp ule v39, v34
;; @008f                               trapz v40, user1
;; @008f                               v41 = iadd v32, v37
;; @008f                               store notrap aligned v30, v41
;;                                     v111 = stack_addr.i64 ss0
;;                                     v97 = load.i32 notrap v111
;; @008f                               store notrap aligned v97, v15
;;                                     v112 = iconst.i64 4
;; @008f                               v42 = iadd.i64 v15, v112  ; v112 = 4
;; @008f                               store notrap aligned v42, v14
;; @008f                               jump block5
;;
;;                                 block3 cold:
;;                                     v113 = stack_addr.i64 ss0
;;                                     v96 = load.i32 notrap v113
;; @008f                               v44 = call fn0(v0, v96), stack_map=[i32 @ ss0+0]
;; @008f                               jump block5
;;
;;                                 block5:
;;                                     v114 = iconst.i64 112
;; @0091                               v46 = iadd.i64 v0, v114  ; v114 = 112
;; @0091                               v47 = load.i32 notrap aligned readonly can_move v46
;;                                     v115 = stack_addr.i64 ss1
;;                                     store notrap v47, v115
;;                                     v116 = stack_addr.i64 ss1
;;                                     v95 = load.i32 notrap v116
;;                                     v117 = iconst.i32 1
;; @0091                               v48 = band v95, v117  ; v117 = 1
;;                                     v118 = stack_addr.i64 ss1
;;                                     v94 = load.i32 notrap v118
;;                                     v119 = iconst.i32 0
;; @0091                               v49 = icmp eq v94, v119  ; v119 = 0
;; @0091                               v50 = uextend.i32 v49
;; @0091                               v51 = bor v48, v50
;; @0091                               brif v51, block9, block6
;;
;;                                 block6:
;; @0091                               v53 = load.i64 notrap aligned readonly v0+56
;; @0091                               v54 = load.i64 notrap aligned v53
;; @0091                               v55 = load.i64 notrap aligned v53+8
;; @0091                               v56 = icmp eq v54, v55
;; @0091                               brif v56, block7, block8
;;
;;                                 block8:
;; @0091                               v58 = load.i64 notrap aligned readonly can_move v0+40
;; @0091                               v60 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v120 = stack_addr.i64 ss1
;;                                     v93 = load.i32 notrap v120
;; @0091                               v61 = uextend.i64 v93
;; @0091                               v62 = iconst.i64 8
;; @0091                               v63 = uadd_overflow_trap v61, v62, user1  ; v62 = 8
;; @0091                               v64 = iconst.i64 8
;; @0091                               v65 = uadd_overflow_trap v63, v64, user1  ; v64 = 8
;; @0091                               v66 = icmp ule v65, v60
;; @0091                               trapz v66, user1
;; @0091                               v67 = iadd v58, v63
;; @0091                               v68 = load.i64 notrap aligned v67
;;                                     v121 = iconst.i64 1
;; @0091                               v69 = iadd v68, v121  ; v121 = 1
;; @0091                               v71 = load.i64 notrap aligned readonly can_move v0+40
;; @0091                               v73 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v122 = stack_addr.i64 ss1
;;                                     v92 = load.i32 notrap v122
;; @0091                               v74 = uextend.i64 v92
;; @0091                               v75 = iconst.i64 8
;; @0091                               v76 = uadd_overflow_trap v74, v75, user1  ; v75 = 8
;; @0091                               v77 = iconst.i64 8
;; @0091                               v78 = uadd_overflow_trap v76, v77, user1  ; v77 = 8
;; @0091                               v79 = icmp ule v78, v73
;; @0091                               trapz v79, user1
;; @0091                               v80 = iadd v71, v76
;; @0091                               store notrap aligned v69, v80
;;                                     v123 = stack_addr.i64 ss1
;;                                     v91 = load.i32 notrap v123
;; @0091                               store notrap aligned v91, v54
;;                                     v124 = iconst.i64 4
;; @0091                               v81 = iadd.i64 v54, v124  ; v124 = 4
;; @0091                               store notrap aligned v81, v53
;; @0091                               jump block9
;;
;;                                 block7 cold:
;;                                     v125 = stack_addr.i64 ss1
;;                                     v90 = load.i32 notrap v125
;; @0091                               v83 = call fn0(v0, v90), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v85 = load.i64 notrap aligned table v0+128
;; @0095                               v87 = load.i64 notrap aligned table v0+144
;;                                     v126 = stack_addr.i64 ss0
;;                                     v88 = load.i32 notrap v126
;;                                     v127 = stack_addr.i64 ss1
;;                                     v89 = load.i32 notrap v127
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v88, v89, v85, v87
;; }
