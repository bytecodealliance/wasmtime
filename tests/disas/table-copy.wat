;;! target = "x86_64"

(module $n
  (table $t (import "m" "t") 6 funcref)

  (func $i (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 3))
  (func $j (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 4))
  (func $k (param i32 i32 i32 i32 i32 i32) (result i32) (local.get 5))

  (table $u (export "u") funcref (elem $i $j $k $i $j $k))

  (func (export "copy_to_t_from_u") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $t $u)

  (func (export "copy_to_u_from_t") (param i32 i32 i32 i32) (result i32)
    local.get 0
    local.get 1
    local.get 2
    local.get 3
    table.copy $u $t))

;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @007b                               jump block1
;;
;;                                 block1:
;; @007b                               return v5
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @0080                               jump block1
;;
;;                                 block1:
;; @0080                               return v6
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32, v7: i32):
;; @0085                               jump block1
;;
;;                                 block1:
;; @0085                               return v7
;; }
;;
;; function u0:3(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4
;;     gv6 = load.i64 notrap aligned gv4+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0090                               v111 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v7 = load.i64 notrap aligned v111+8
;; @0090                               v8 = ireduce.i32 v7
;; @0090                               v9 = uextend.i64 v8
;; @0090                               v10 = uextend.i64 v3
;; @0090                               v11 = uextend.i64 v5
;;                                     v110 = iconst.i64 1
;; @0090                               v12 = imul v11, v110  ; v110 = 1
;; @0090                               v13 = iadd v10, v12
;; @0090                               v14 = icmp ugt v13, v9
;; @0090                               trapnz v14, user6
;; @0090                               v108 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v15 = load.i64 notrap aligned v108
;; @0090                               v16 = uextend.i64 v3
;;                                     v107 = iconst.i64 8
;; @0090                               v17 = imul v16, v107  ; v107 = 8
;; @0090                               v18 = iadd v15, v17
;; @0090                               v19 = iconst.i32 6
;; @0090                               v20 = uextend.i64 v19  ; v19 = 6
;; @0090                               v21 = uextend.i64 v4
;; @0090                               v22 = uextend.i64 v5
;;                                     v106 = iconst.i64 1
;; @0090                               v23 = imul v22, v106  ; v106 = 1
;; @0090                               v24 = iadd v21, v23
;; @0090                               v25 = icmp ugt v24, v20
;; @0090                               trapnz v25, user6
;; @0090                               v26 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v27 = uextend.i64 v4
;;                                     v104 = iconst.i64 8
;; @0090                               v28 = imul v27, v104  ; v104 = 8
;; @0090                               v29 = iadd v26, v28
;; @0090                               v30 = uextend.i64 v5
;;                                     v103 = iconst.i64 8
;; @0090                               v31 = imul v30, v103  ; v103 = 8
;;                                     v102 = iconst.i64 8
;; @0090                               v32 = imul v30, v102  ; v102 = 8
;; @0090                               brif v30, block2, block5
;;
;;                                 block2:
;; @0090                               v33 = icmp.i64 ult v18, v29
;;                                     v101 = iconst.i64 8
;; @0090                               v34 = imul.i64 v30, v101  ; v101 = 8
;;                                     v100 = iconst.i64 8
;; @0090                               v35 = imul.i64 v30, v100  ; v100 = 8
;; @0090                               v36 = iadd.i64 v18, v34
;; @0090                               v37 = iadd.i64 v29, v35
;; @0090                               v38 = ireduce.i32 v30
;; @0090                               v39 = iadd.i32 v4, v38
;; @0090                               brif v33, block3(v18, v29, v4), block4(v36, v37, v39)
;;
;;                                 block3(v40: i64, v41: i64, v42: i32):
;; @0090                               v43 = iconst.i32 6
;; @0090                               v44 = icmp uge v42, v43  ; v43 = 6
;; @0090                               v45 = uextend.i64 v42
;; @0090                               v46 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v98 = iconst.i64 3
;; @0090                               v47 = ishl v45, v98  ; v98 = 3
;; @0090                               v48 = iadd v46, v47
;; @0090                               v49 = iconst.i64 0
;; @0090                               v50 = select_spectre_guard v44, v49, v48  ; v49 = 0
;; @0090                               v51 = load.i64 user6 aligned region0 v50
;;                                     v97 = iconst.i64 -2
;; @0090                               v52 = band v51, v97  ; v97 = -2
;; @0090                               brif v51, block7(v52), block6
;;
;;                                 block4(v66: i64, v67: i64, v68: i32):
;; @0090                               v69 = iconst.i64 8
;; @0090                               v70 = isub v66, v69  ; v69 = 8
;; @0090                               v71 = iconst.i64 8
;; @0090                               v72 = isub v67, v71  ; v71 = 8
;; @0090                               v73 = iconst.i32 1
;; @0090                               v74 = isub v68, v73  ; v73 = 1
;; @0090                               v75 = iconst.i32 6
;; @0090                               v76 = icmp uge v74, v75  ; v75 = 6
;; @0090                               v77 = uextend.i64 v74
;; @0090                               v78 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v95 = iconst.i64 3
;; @0090                               v79 = ishl v77, v95  ; v95 = 3
;; @0090                               v80 = iadd v78, v79
;; @0090                               v81 = iconst.i64 0
;; @0090                               v82 = select_spectre_guard v76, v81, v80  ; v81 = 0
;; @0090                               v83 = load.i64 user6 aligned region0 v82
;;                                     v94 = iconst.i64 -2
;; @0090                               v84 = band v83, v94  ; v94 = -2
;; @0090                               brif v83, block9(v84), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v54 = iconst.i32 1
;; @0090                               v56 = uextend.i64 v42
;; @0090                               v57 = call fn0(v0, v54, v56)  ; v54 = 1
;; @0090                               jump block7(v57)
;;
;;                                 block7(v53: i64):
;;                                     v93 = iconst.i64 1
;; @0090                               v58 = bor v53, v93  ; v93 = 1
;; @0090                               store notrap aligned v58, v40
;; @0090                               v59 = iconst.i64 8
;; @0090                               v60 = iadd.i64 v40, v59  ; v59 = 8
;; @0090                               v61 = iconst.i64 8
;; @0090                               v62 = iadd.i64 v41, v61  ; v61 = 8
;; @0090                               v63 = iconst.i32 1
;; @0090                               v64 = iadd.i32 v42, v63  ; v63 = 1
;; @0090                               v65 = icmp eq v62, v37
;; @0090                               brif v65, block5, block3(v60, v62, v64)
;;
;;                                 block8 cold:
;; @0090                               v86 = iconst.i32 1
;; @0090                               v88 = uextend.i64 v74
;; @0090                               v89 = call fn0(v0, v86, v88)  ; v86 = 1
;; @0090                               jump block9(v89)
;;
;;                                 block9(v85: i64):
;;                                     v92 = iconst.i64 1
;; @0090                               v90 = bor v85, v92  ; v92 = 1
;; @0090                               store notrap aligned v90, v70
;; @0090                               v91 = icmp.i64 eq v72, v29
;; @0090                               brif v91, block5, block4(v70, v72, v74)
;;
;;                                 block1:
;; @0094                               return v2
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 1073741824 "ImportedTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv6 = load.i64 notrap aligned gv5
;;     gv7 = load.i64 notrap aligned gv5+8
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009f                               v7 = iconst.i32 6
;; @009f                               v8 = uextend.i64 v7  ; v7 = 6
;; @009f                               v9 = uextend.i64 v3
;; @009f                               v10 = uextend.i64 v5
;;                                     v120 = iconst.i64 1
;; @009f                               v11 = imul v10, v120  ; v120 = 1
;; @009f                               v12 = iadd v9, v11
;; @009f                               v13 = icmp ugt v12, v8
;; @009f                               trapnz v13, user6
;; @009f                               v14 = load.i64 notrap aligned readonly can_move v0+72
;; @009f                               v15 = uextend.i64 v3
;;                                     v118 = iconst.i64 8
;; @009f                               v16 = imul v15, v118  ; v118 = 8
;; @009f                               v17 = iadd v14, v16
;; @009f                               v116 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v18 = load.i64 notrap aligned v116+8
;; @009f                               v19 = ireduce.i32 v18
;; @009f                               v20 = uextend.i64 v19
;; @009f                               v21 = uextend.i64 v4
;; @009f                               v22 = uextend.i64 v5
;;                                     v115 = iconst.i64 1
;; @009f                               v23 = imul v22, v115  ; v115 = 1
;; @009f                               v24 = iadd v21, v23
;; @009f                               v25 = icmp ugt v24, v20
;; @009f                               trapnz v25, user6
;; @009f                               v113 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v26 = load.i64 notrap aligned v113
;; @009f                               v27 = uextend.i64 v4
;;                                     v112 = iconst.i64 8
;; @009f                               v28 = imul v27, v112  ; v112 = 8
;; @009f                               v29 = iadd v26, v28
;; @009f                               v30 = uextend.i64 v5
;;                                     v111 = iconst.i64 8
;; @009f                               v31 = imul v30, v111  ; v111 = 8
;;                                     v110 = iconst.i64 8
;; @009f                               v32 = imul v30, v110  ; v110 = 8
;; @009f                               brif v30, block2, block5
;;
;;                                 block2:
;; @009f                               v33 = icmp.i64 ult v17, v29
;;                                     v109 = iconst.i64 8
;; @009f                               v34 = imul.i64 v30, v109  ; v109 = 8
;;                                     v108 = iconst.i64 8
;; @009f                               v35 = imul.i64 v30, v108  ; v108 = 8
;; @009f                               v36 = iadd.i64 v17, v34
;; @009f                               v37 = iadd.i64 v29, v35
;; @009f                               v38 = ireduce.i32 v30
;; @009f                               v39 = iadd.i32 v4, v38
;; @009f                               brif v33, block3(v17, v29, v4), block4(v36, v37, v39)
;;
;;                                 block3(v40: i64, v41: i64, v42: i32):
;; @009f                               v106 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v43 = load.i64 notrap aligned v106+8
;; @009f                               v44 = ireduce.i32 v43
;; @009f                               v45 = icmp uge v42, v44
;; @009f                               v46 = uextend.i64 v42
;; @009f                               v104 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v47 = load.i64 notrap aligned v104
;;                                     v103 = iconst.i64 3
;; @009f                               v48 = ishl v46, v103  ; v103 = 3
;; @009f                               v49 = iadd v47, v48
;; @009f                               v50 = iconst.i64 0
;; @009f                               v51 = select_spectre_guard v45, v50, v49  ; v50 = 0
;; @009f                               v52 = load.i64 user6 aligned region0 v51
;;                                     v102 = iconst.i64 -2
;; @009f                               v53 = band v52, v102  ; v102 = -2
;; @009f                               brif v52, block7(v53), block6
;;
;;                                 block4(v67: i64, v68: i64, v69: i32):
;; @009f                               v70 = iconst.i64 8
;; @009f                               v71 = isub v67, v70  ; v70 = 8
;; @009f                               v72 = iconst.i64 8
;; @009f                               v73 = isub v68, v72  ; v72 = 8
;; @009f                               v74 = iconst.i32 1
;; @009f                               v75 = isub v69, v74  ; v74 = 1
;; @009f                               v100 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v76 = load.i64 notrap aligned v100+8
;; @009f                               v77 = ireduce.i32 v76
;; @009f                               v78 = icmp uge v75, v77
;; @009f                               v79 = uextend.i64 v75
;; @009f                               v98 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v80 = load.i64 notrap aligned v98
;;                                     v97 = iconst.i64 3
;; @009f                               v81 = ishl v79, v97  ; v97 = 3
;; @009f                               v82 = iadd v80, v81
;; @009f                               v83 = iconst.i64 0
;; @009f                               v84 = select_spectre_guard v78, v83, v82  ; v83 = 0
;; @009f                               v85 = load.i64 user6 aligned region0 v84
;;                                     v96 = iconst.i64 -2
;; @009f                               v86 = band v85, v96  ; v96 = -2
;; @009f                               brif v85, block9(v86), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v55 = iconst.i32 0
;; @009f                               v57 = uextend.i64 v42
;; @009f                               v58 = call fn0(v0, v55, v57)  ; v55 = 0
;; @009f                               jump block7(v58)
;;
;;                                 block7(v54: i64):
;;                                     v95 = iconst.i64 1
;; @009f                               v59 = bor v54, v95  ; v95 = 1
;; @009f                               store notrap aligned v59, v40
;; @009f                               v60 = iconst.i64 8
;; @009f                               v61 = iadd.i64 v40, v60  ; v60 = 8
;; @009f                               v62 = iconst.i64 8
;; @009f                               v63 = iadd.i64 v41, v62  ; v62 = 8
;; @009f                               v64 = iconst.i32 1
;; @009f                               v65 = iadd.i32 v42, v64  ; v64 = 1
;; @009f                               v66 = icmp eq v63, v37
;; @009f                               brif v66, block5, block3(v61, v63, v65)
;;
;;                                 block8 cold:
;; @009f                               v88 = iconst.i32 0
;; @009f                               v90 = uextend.i64 v75
;; @009f                               v91 = call fn0(v0, v88, v90)  ; v88 = 0
;; @009f                               jump block9(v91)
;;
;;                                 block9(v87: i64):
;;                                     v94 = iconst.i64 1
;; @009f                               v92 = bor v87, v94  ; v94 = 1
;; @009f                               store notrap aligned v92, v71
;; @009f                               v93 = icmp.i64 eq v73, v29
;; @009f                               brif v93, block5, block4(v71, v73, v75)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
