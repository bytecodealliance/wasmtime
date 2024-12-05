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
;;     sig0 = (i64 vmctx, i32) -> i32 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v6 = global_value.i64 gv3
;; @008f                               v7 = iadd_imm v6, 112
;; @008f                               v8 = load.i32 notrap aligned v7
;;                                     stack_store v8, ss0
;;                                     v93 = stack_load.i32 ss0
;; @008f                               v9 = icmp_imm eq v93, 0
;; @008f                               brif v9, block5, block2
;;
;;                                 block2:
;; @008f                               v10 = global_value.i64 gv3
;; @008f                               v11 = load.i64 notrap aligned readonly v10+56
;; @008f                               v12 = load.i64 notrap aligned v11
;; @008f                               v13 = load.i64 notrap aligned v11+8
;; @008f                               v14 = icmp eq v12, v13
;; @008f                               brif v14, block3, block4
;;
;;                                 block4:
;; @008f                               v15 = global_value.i64 gv3
;; @008f                               v16 = load.i64 notrap aligned readonly v15+40
;; @008f                               v17 = global_value.i64 gv3
;; @008f                               v18 = load.i64 notrap aligned readonly v17+48
;;                                     v92 = stack_load.i32 ss0
;; @008f                               v19 = uextend.i64 v92
;; @008f                               v20 = iconst.i64 8
;; @008f                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @008f                               v22 = iconst.i64 8
;; @008f                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @008f                               v24 = icmp ule v23, v18
;; @008f                               trapz v24, user1
;; @008f                               v25 = iadd v16, v21
;; @008f                               v26 = load.i64 notrap aligned v25
;; @008f                               v27 = iadd_imm v26, 1
;; @008f                               v28 = global_value.i64 gv3
;; @008f                               v29 = load.i64 notrap aligned readonly v28+40
;; @008f                               v30 = global_value.i64 gv3
;; @008f                               v31 = load.i64 notrap aligned readonly v30+48
;;                                     v91 = stack_load.i32 ss0
;; @008f                               v32 = uextend.i64 v91
;; @008f                               v33 = iconst.i64 8
;; @008f                               v34 = uadd_overflow_trap v32, v33, user1  ; v33 = 8
;; @008f                               v35 = iconst.i64 8
;; @008f                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @008f                               v37 = icmp ule v36, v31
;; @008f                               trapz v37, user1
;; @008f                               v38 = iadd v29, v34
;; @008f                               store notrap aligned v27, v38
;;                                     v90 = stack_load.i32 ss0
;; @008f                               store notrap aligned v90, v12
;; @008f                               v39 = iadd_imm.i64 v12, 4
;; @008f                               store notrap aligned v39, v11
;; @008f                               jump block5
;;
;;                                 block3 cold:
;; @008f                               v40 = global_value.i64 gv3
;;                                     v89 = stack_load.i32 ss0
;; @008f                               v41 = call fn0(v40, v89), stack_map=[i32 @ ss0+0]
;; @008f                               jump block5
;;
;;                                 block5:
;; @0091                               v42 = global_value.i64 gv3
;; @0091                               v43 = iadd_imm v42, 128
;; @0091                               v44 = load.i32 notrap aligned v43
;;                                     stack_store v44, ss1
;;                                     v88 = stack_load.i32 ss1
;; @0091                               v45 = icmp_imm eq v88, 0
;; @0091                               brif v45, block9, block6
;;
;;                                 block6:
;; @0091                               v46 = global_value.i64 gv3
;; @0091                               v47 = load.i64 notrap aligned readonly v46+56
;; @0091                               v48 = load.i64 notrap aligned v47
;; @0091                               v49 = load.i64 notrap aligned v47+8
;; @0091                               v50 = icmp eq v48, v49
;; @0091                               brif v50, block7, block8
;;
;;                                 block8:
;; @0091                               v51 = global_value.i64 gv3
;; @0091                               v52 = load.i64 notrap aligned readonly v51+40
;; @0091                               v53 = global_value.i64 gv3
;; @0091                               v54 = load.i64 notrap aligned readonly v53+48
;;                                     v87 = stack_load.i32 ss1
;; @0091                               v55 = uextend.i64 v87
;; @0091                               v56 = iconst.i64 8
;; @0091                               v57 = uadd_overflow_trap v55, v56, user1  ; v56 = 8
;; @0091                               v58 = iconst.i64 8
;; @0091                               v59 = uadd_overflow_trap v57, v58, user1  ; v58 = 8
;; @0091                               v60 = icmp ule v59, v54
;; @0091                               trapz v60, user1
;; @0091                               v61 = iadd v52, v57
;; @0091                               v62 = load.i64 notrap aligned v61
;; @0091                               v63 = iadd_imm v62, 1
;; @0091                               v64 = global_value.i64 gv3
;; @0091                               v65 = load.i64 notrap aligned readonly v64+40
;; @0091                               v66 = global_value.i64 gv3
;; @0091                               v67 = load.i64 notrap aligned readonly v66+48
;;                                     v86 = stack_load.i32 ss1
;; @0091                               v68 = uextend.i64 v86
;; @0091                               v69 = iconst.i64 8
;; @0091                               v70 = uadd_overflow_trap v68, v69, user1  ; v69 = 8
;; @0091                               v71 = iconst.i64 8
;; @0091                               v72 = uadd_overflow_trap v70, v71, user1  ; v71 = 8
;; @0091                               v73 = icmp ule v72, v67
;; @0091                               trapz v73, user1
;; @0091                               v74 = iadd v65, v70
;; @0091                               store notrap aligned v63, v74
;;                                     v85 = stack_load.i32 ss1
;; @0091                               store notrap aligned v85, v48
;; @0091                               v75 = iadd_imm.i64 v48, 4
;; @0091                               store notrap aligned v75, v47
;; @0091                               jump block9
;;
;;                                 block7 cold:
;; @0091                               v76 = global_value.i64 gv3
;;                                     v84 = stack_load.i32 ss1
;; @0091                               v77 = call fn0(v76, v84), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v78 = global_value.i64 gv3
;; @0093                               v79 = load.i64 notrap aligned table v78+144
;; @0095                               v80 = global_value.i64 gv3
;; @0095                               v81 = load.i64 notrap aligned table v80+160
;;                                     v82 = stack_load.i32 ss0
;;                                     v83 = stack_load.i32 ss1
;; @0097                               jump block1(v82, v83, v79, v81)
;;
;;                                 block1(v2: i32, v3: i32, v4: i64, v5: i64):
;; @0097                               return v2, v3, v4, v5
;; }
