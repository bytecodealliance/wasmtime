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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v6 = global_value.i64 gv3
;; @008f                               v7 = iadd_imm v6, 112
;; @008f                               v8 = load.i32 notrap aligned v7
;;                                     stack_store v8, ss0
;;                                     v89 = stack_load.i32 ss0
;; @008f                               v9 = icmp_imm eq v89, 0
;; @008f                               brif v9, block5, block2
;;
;;                                 block2:
;; @008f                               v10 = global_value.i64 gv3
;; @008f                               v11 = load.i64 notrap aligned v10+56
;; @008f                               v12 = load.i64 notrap aligned v11
;; @008f                               v13 = load.i64 notrap aligned v11+8
;; @008f                               v14 = icmp eq v12, v13
;; @008f                               brif v14, block3, block4
;;
;;                                 block4:
;; @008f                               v15 = global_value.i64 gv3
;; @008f                               v16 = load.i64 notrap aligned readonly v15+40
;; @008f                               v17 = load.i64 notrap aligned readonly v15+48
;;                                     v88 = stack_load.i32 ss0
;; @008f                               v18 = uextend.i64 v88
;; @008f                               v19 = iconst.i64 8
;; @008f                               v20 = uadd_overflow_trap v18, v19, user1  ; v19 = 8
;; @008f                               v21 = iconst.i64 8
;; @008f                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 8
;; @008f                               v23 = icmp ule v22, v17
;; @008f                               trapz v23, user1
;; @008f                               v24 = iadd v16, v20
;; @008f                               v25 = load.i64 notrap aligned v24
;; @008f                               v26 = iadd_imm v25, 1
;; @008f                               v27 = global_value.i64 gv3
;; @008f                               v28 = load.i64 notrap aligned readonly v27+40
;; @008f                               v29 = load.i64 notrap aligned readonly v27+48
;;                                     v87 = stack_load.i32 ss0
;; @008f                               v30 = uextend.i64 v87
;; @008f                               v31 = iconst.i64 8
;; @008f                               v32 = uadd_overflow_trap v30, v31, user1  ; v31 = 8
;; @008f                               v33 = iconst.i64 8
;; @008f                               v34 = uadd_overflow_trap v32, v33, user1  ; v33 = 8
;; @008f                               v35 = icmp ule v34, v29
;; @008f                               trapz v35, user1
;; @008f                               v36 = iadd v28, v32
;; @008f                               store notrap aligned v26, v36
;;                                     v86 = stack_load.i32 ss0
;; @008f                               store notrap aligned v86, v12
;; @008f                               v37 = iadd_imm.i64 v12, 4
;; @008f                               store notrap aligned v37, v11
;; @008f                               jump block5
;;
;;                                 block3 cold:
;; @008f                               v38 = global_value.i64 gv3
;;                                     v85 = stack_load.i32 ss0
;; @008f                               v39 = call fn0(v38, v85), stack_map=[i32 @ ss0+0]
;; @008f                               jump block5
;;
;;                                 block5:
;; @0091                               v40 = global_value.i64 gv3
;; @0091                               v41 = iadd_imm v40, 128
;; @0091                               v42 = load.i32 notrap aligned v41
;;                                     stack_store v42, ss1
;;                                     v84 = stack_load.i32 ss1
;; @0091                               v43 = icmp_imm eq v84, 0
;; @0091                               brif v43, block9, block6
;;
;;                                 block6:
;; @0091                               v44 = global_value.i64 gv3
;; @0091                               v45 = load.i64 notrap aligned v44+56
;; @0091                               v46 = load.i64 notrap aligned v45
;; @0091                               v47 = load.i64 notrap aligned v45+8
;; @0091                               v48 = icmp eq v46, v47
;; @0091                               brif v48, block7, block8
;;
;;                                 block8:
;; @0091                               v49 = global_value.i64 gv3
;; @0091                               v50 = load.i64 notrap aligned readonly v49+40
;; @0091                               v51 = load.i64 notrap aligned readonly v49+48
;;                                     v83 = stack_load.i32 ss1
;; @0091                               v52 = uextend.i64 v83
;; @0091                               v53 = iconst.i64 8
;; @0091                               v54 = uadd_overflow_trap v52, v53, user1  ; v53 = 8
;; @0091                               v55 = iconst.i64 8
;; @0091                               v56 = uadd_overflow_trap v54, v55, user1  ; v55 = 8
;; @0091                               v57 = icmp ule v56, v51
;; @0091                               trapz v57, user1
;; @0091                               v58 = iadd v50, v54
;; @0091                               v59 = load.i64 notrap aligned v58
;; @0091                               v60 = iadd_imm v59, 1
;; @0091                               v61 = global_value.i64 gv3
;; @0091                               v62 = load.i64 notrap aligned readonly v61+40
;; @0091                               v63 = load.i64 notrap aligned readonly v61+48
;;                                     v82 = stack_load.i32 ss1
;; @0091                               v64 = uextend.i64 v82
;; @0091                               v65 = iconst.i64 8
;; @0091                               v66 = uadd_overflow_trap v64, v65, user1  ; v65 = 8
;; @0091                               v67 = iconst.i64 8
;; @0091                               v68 = uadd_overflow_trap v66, v67, user1  ; v67 = 8
;; @0091                               v69 = icmp ule v68, v63
;; @0091                               trapz v69, user1
;; @0091                               v70 = iadd v62, v66
;; @0091                               store notrap aligned v60, v70
;;                                     v81 = stack_load.i32 ss1
;; @0091                               store notrap aligned v81, v46
;; @0091                               v71 = iadd_imm.i64 v46, 4
;; @0091                               store notrap aligned v71, v45
;; @0091                               jump block9
;;
;;                                 block7 cold:
;; @0091                               v72 = global_value.i64 gv3
;;                                     v80 = stack_load.i32 ss1
;; @0091                               v73 = call fn0(v72, v80), stack_map=[i32 @ ss0+0, i32 @ ss1+0]
;; @0091                               jump block9
;;
;;                                 block9:
;; @0093                               v74 = global_value.i64 gv3
;; @0093                               v75 = load.i64 notrap aligned table v74+144
;; @0095                               v76 = global_value.i64 gv3
;; @0095                               v77 = load.i64 notrap aligned table v76+160
;;                                     v78 = stack_load.i32 ss0
;;                                     v79 = stack_load.i32 ss1
;; @0097                               jump block1(v78, v79, v75, v77)
;;
;;                                 block1(v2: i32, v3: i32, v4: i64, v5: i64):
;; @0097                               return v2, v3, v4, v5
;; }
