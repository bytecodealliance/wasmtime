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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v111 = iconst.i64 80
;; @008f                               v7 = iadd v0, v111  ; v111 = 80
;; @008f                               v8 = load.i32 notrap aligned readonly can_move v7
;;                                     v110 = stack_addr.i64 ss0
;;                                     store notrap v8, v110
;;                                     v109 = stack_addr.i64 ss0
;;                                     v77 = load.i32 notrap v109
;;                                     v108 = iconst.i32 1
;; @008f                               v9 = band v77, v108  ; v108 = 1
;;                                     v107 = stack_addr.i64 ss0
;;                                     v76 = load.i32 notrap v107
;;                                     v106 = iconst.i32 0
;; @008f                               v10 = icmp eq v76, v106  ; v106 = 0
;; @008f                               v11 = uextend.i32 v10
;; @008f                               v12 = bor v9, v11
;; @008f                               brif v12, block5, block2
;;
;;                                 block2:
;; @008f                               v14 = load.i64 notrap aligned readonly v0+40
;; @008f                               v15 = load.i64 notrap aligned v14
;; @008f                               v16 = load.i64 notrap aligned v14+8
;; @008f                               v17 = icmp eq v15, v16
;; @008f                               brif v17, block3, block4
;;
;;                                 block4:
;;                                     v105 = stack_addr.i64 ss0
;;                                     v75 = load.i32 notrap v105
;; @008f                               v18 = uextend.i64 v75
;; @008f                               v103 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v19 = load.i64 notrap aligned readonly can_move v103+24
;; @008f                               v20 = iadd v19, v18
;; @008f                               v21 = iconst.i64 8
;; @008f                               v22 = iadd v20, v21  ; v21 = 8
;; @008f                               v23 = load.i64 notrap aligned v22
;;                                     v102 = iconst.i64 1
;; @008f                               v24 = iadd v23, v102  ; v102 = 1
;;                                     v101 = stack_addr.i64 ss0
;;                                     v74 = load.i32 notrap v101
;; @008f                               v25 = uextend.i64 v74
;; @008f                               v99 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v26 = load.i64 notrap aligned readonly can_move v99+24
;; @008f                               v27 = iadd v26, v25
;; @008f                               v28 = iconst.i64 8
;; @008f                               v29 = iadd v27, v28  ; v28 = 8
;; @008f                               store notrap aligned v24, v29
;;                                     v98 = stack_addr.i64 ss0
;;                                     v73 = load.i32 notrap v98
;; @008f                               store notrap aligned v73, v15
;;                                     v97 = iconst.i64 4
;; @008f                               v30 = iadd.i64 v15, v97  ; v97 = 4
;; @008f                               store notrap aligned v30, v14
;; @008f                               jump block5
;;
;;                                 block3 cold:
;;                                     v96 = stack_addr.i64 ss0
;;                                     v72 = load.i32 notrap v96
;; @008f                               v32 = call fn0(v0, v72), stack_map=[i32 @ ss0+0]
;; @008f                               jump block5
;;
;;                                 block5:
;;                                     v95 = iconst.i64 96
;; @0091                               v34 = iadd.i64 v0, v95  ; v95 = 96
;; @0091                               v35 = load.i32 notrap aligned readonly can_move v34
;;                                     v94 = stack_addr.i64 ss1
;;                                     store notrap v35, v94
;;                                     v93 = stack_addr.i64 ss1
;;                                     v71 = load.i32 notrap v93
;;                                     v92 = iconst.i32 1
;; @0091                               v36 = band v71, v92  ; v92 = 1
;;                                     v91 = stack_addr.i64 ss1
;;                                     v70 = load.i32 notrap v91
;;                                     v90 = iconst.i32 0
;; @0091                               v37 = icmp eq v70, v90  ; v90 = 0
;; @0091                               v38 = uextend.i32 v37
;; @0091                               v39 = bor v36, v38
;; @0091                               brif v39, block9, block6
;;
;;                                 block6:
;; @0091                               v41 = load.i64 notrap aligned readonly v0+40
;; @0091                               v42 = load.i64 notrap aligned v41
;; @0091                               v43 = load.i64 notrap aligned v41+8
;; @0091                               v44 = icmp eq v42, v43
;; @0091                               brif v44, block7, block8
;;
;;                                 block8:
;;                                     v89 = stack_addr.i64 ss1
;;                                     v69 = load.i32 notrap v89
;; @0091                               v45 = uextend.i64 v69
;; @0091                               v87 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v46 = load.i64 notrap aligned readonly can_move v87+24
;; @0091                               v47 = iadd v46, v45
;; @0091                               v48 = iconst.i64 8
;; @0091                               v49 = iadd v47, v48  ; v48 = 8
;; @0091                               v50 = load.i64 notrap aligned v49
;;                                     v86 = iconst.i64 1
;; @0091                               v51 = iadd v50, v86  ; v86 = 1
;;                                     v85 = stack_addr.i64 ss1
;;                                     v68 = load.i32 notrap v85
;; @0091                               v52 = uextend.i64 v68
;; @0091                               v83 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v53 = load.i64 notrap aligned readonly can_move v83+24
;; @0091                               v54 = iadd v53, v52
;; @0091                               v55 = iconst.i64 8
;; @0091                               v56 = iadd v54, v55  ; v55 = 8
;; @0091                               store notrap aligned v51, v56
;;                                     v82 = stack_addr.i64 ss1
;;                                     v67 = load.i32 notrap v82
;; @0091                               store notrap aligned v67, v42
;;                                     v81 = iconst.i64 4
;; @0091                               v57 = iadd.i64 v42, v81  ; v81 = 4
;; @0091                               store notrap aligned v57, v41
;; @0091                               jump block9
;;
;;                                 block7 cold:
;;                                     v80 = stack_addr.i64 ss1
;;                                     v66 = load.i32 notrap v80
;; @0091                               v59 = call fn0(v0, v66), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v61 = load.i64 notrap aligned table v0+112
;; @0095                               v63 = load.i64 notrap aligned table v0+128
;;                                     v79 = stack_addr.i64 ss0
;;                                     v64 = load.i32 notrap v79
;;                                     v78 = stack_addr.i64 ss1
;;                                     v65 = load.i32 notrap v78
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v64, v65, v61, v63
;; }
