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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned gv4
;;     gv6 = load.i64 notrap aligned gv4+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0090                               v102 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v7 = load.i64 notrap aligned v102+8
;; @0090                               v8 = ireduce.i32 v7
;; @0090                               v9 = uextend.i64 v8
;; @0090                               v10 = uextend.i64 v3
;; @0090                               v11 = uextend.i64 v5
;;                                     v101 = iconst.i64 1
;; @0090                               v12 = imul v11, v101  ; v101 = 1
;; @0090                               v13 = iadd v10, v12
;; @0090                               v14 = icmp ugt v13, v9
;; @0090                               trapnz v14, user6
;; @0090                               v99 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v15 = load.i64 notrap aligned v99
;; @0090                               v16 = uextend.i64 v3
;;                                     v98 = iconst.i64 8
;; @0090                               v17 = imul v16, v98  ; v98 = 8
;; @0090                               v18 = iadd v15, v17
;; @0090                               v19 = iconst.i32 6
;; @0090                               v20 = uextend.i64 v19  ; v19 = 6
;; @0090                               v21 = uextend.i64 v4
;; @0090                               v22 = uextend.i64 v5
;;                                     v97 = iconst.i64 1
;; @0090                               v23 = imul v22, v97  ; v97 = 1
;; @0090                               v24 = iadd v21, v23
;; @0090                               v25 = icmp ugt v24, v20
;; @0090                               trapnz v25, user6
;; @0090                               v26 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v27 = uextend.i64 v4
;;                                     v95 = iconst.i64 8
;; @0090                               v28 = imul v27, v95  ; v95 = 8
;; @0090                               v29 = iadd v26, v28
;; @0090                               v30 = uextend.i64 v5
;; @0090                               v31 = iconst.i64 8
;; @0090                               v32 = imul v31, v30  ; v31 = 8
;; @0090                               brif v30, block2, block5
;;
;;                                 block2:
;; @0090                               v33 = icmp.i64 ult v18, v29
;; @0090                               v34 = imul.i64 v30, v31  ; v31 = 8
;; @0090                               v35 = iadd.i64 v18, v34
;; @0090                               v36 = iadd.i64 v29, v34
;; @0090                               v37 = ireduce.i32 v30
;; @0090                               v38 = iadd.i32 v4, v37
;; @0090                               brif v33, block3(v18, v29, v4), block4(v35, v36, v38)
;;
;;                                 block3(v39: i64, v40: i64, v41: i32):
;; @0090                               v42 = iconst.i32 6
;; @0090                               v43 = icmp uge v41, v42  ; v42 = 6
;; @0090                               v44 = uextend.i64 v41
;; @0090                               v45 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v93 = iconst.i64 3
;; @0090                               v46 = ishl v44, v93  ; v93 = 3
;; @0090                               v47 = iadd v45, v46
;; @0090                               v48 = iconst.i64 0
;; @0090                               v49 = select_spectre_guard v43, v48, v47  ; v48 = 0
;; @0090                               v50 = load.i64 user6 aligned table v49
;;                                     v92 = iconst.i64 -2
;; @0090                               v51 = band v50, v92  ; v92 = -2
;; @0090                               brif v50, block7(v51), block6
;;
;;                                 block4(v62: i64, v63: i64, v64: i32):
;; @0090                               v65 = isub v62, v31  ; v31 = 8
;; @0090                               v66 = isub v63, v31  ; v31 = 8
;; @0090                               v67 = iconst.i32 1
;; @0090                               v68 = isub v64, v67  ; v67 = 1
;; @0090                               v69 = iconst.i32 6
;; @0090                               v70 = icmp uge v68, v69  ; v69 = 6
;; @0090                               v71 = uextend.i64 v68
;; @0090                               v72 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v90 = iconst.i64 3
;; @0090                               v73 = ishl v71, v90  ; v90 = 3
;; @0090                               v74 = iadd v72, v73
;; @0090                               v75 = iconst.i64 0
;; @0090                               v76 = select_spectre_guard v70, v75, v74  ; v75 = 0
;; @0090                               v77 = load.i64 user6 aligned table v76
;;                                     v89 = iconst.i64 -2
;; @0090                               v78 = band v77, v89  ; v89 = -2
;; @0090                               brif v77, block9(v78), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v53 = iconst.i32 1
;; @0090                               v55 = uextend.i64 v41
;; @0090                               v56 = call fn0(v0, v53, v55)  ; v53 = 1
;; @0090                               jump block7(v56)
;;
;;                                 block7(v52: i64):
;;                                     v88 = iconst.i64 1
;; @0090                               v57 = bor v52, v88  ; v88 = 1
;; @0090                               store notrap aligned v57, v39
;; @0090                               v58 = iadd.i64 v39, v31  ; v31 = 8
;; @0090                               v59 = iadd.i64 v40, v31  ; v31 = 8
;;                                     v87 = iconst.i32 1
;; @0090                               v60 = iadd.i32 v41, v87  ; v87 = 1
;; @0090                               v61 = icmp eq v59, v36
;; @0090                               brif v61, block5, block3(v58, v59, v60)
;;
;;                                 block8 cold:
;; @0090                               v80 = iconst.i32 1
;; @0090                               v82 = uextend.i64 v68
;; @0090                               v83 = call fn0(v0, v80, v82)  ; v80 = 1
;; @0090                               jump block9(v83)
;;
;;                                 block9(v79: i64):
;;                                     v86 = iconst.i64 1
;; @0090                               v84 = bor v79, v86  ; v86 = 1
;; @0090                               store notrap aligned v84, v65
;; @0090                               v85 = icmp.i64 eq v66, v29
;; @0090                               brif v85, block5, block4(v65, v66, v68)
;;
;;                                 block1:
;; @0094                               return v2
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv6 = load.i64 notrap aligned gv5
;;     gv7 = load.i64 notrap aligned gv5+8
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:6 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @009f                               v7 = iconst.i32 6
;; @009f                               v8 = uextend.i64 v7  ; v7 = 6
;; @009f                               v9 = uextend.i64 v3
;; @009f                               v10 = uextend.i64 v5
;;                                     v111 = iconst.i64 1
;; @009f                               v11 = imul v10, v111  ; v111 = 1
;; @009f                               v12 = iadd v9, v11
;; @009f                               v13 = icmp ugt v12, v8
;; @009f                               trapnz v13, user6
;; @009f                               v14 = load.i64 notrap aligned readonly can_move v0+72
;; @009f                               v15 = uextend.i64 v3
;;                                     v109 = iconst.i64 8
;; @009f                               v16 = imul v15, v109  ; v109 = 8
;; @009f                               v17 = iadd v14, v16
;; @009f                               v107 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v18 = load.i64 notrap aligned v107+8
;; @009f                               v19 = ireduce.i32 v18
;; @009f                               v20 = uextend.i64 v19
;; @009f                               v21 = uextend.i64 v4
;; @009f                               v22 = uextend.i64 v5
;;                                     v106 = iconst.i64 1
;; @009f                               v23 = imul v22, v106  ; v106 = 1
;; @009f                               v24 = iadd v21, v23
;; @009f                               v25 = icmp ugt v24, v20
;; @009f                               trapnz v25, user6
;; @009f                               v104 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v26 = load.i64 notrap aligned v104
;; @009f                               v27 = uextend.i64 v4
;;                                     v103 = iconst.i64 8
;; @009f                               v28 = imul v27, v103  ; v103 = 8
;; @009f                               v29 = iadd v26, v28
;; @009f                               v30 = uextend.i64 v5
;; @009f                               v31 = iconst.i64 8
;; @009f                               v32 = imul v31, v30  ; v31 = 8
;; @009f                               brif v30, block2, block5
;;
;;                                 block2:
;; @009f                               v33 = icmp.i64 ult v17, v29
;; @009f                               v34 = imul.i64 v30, v31  ; v31 = 8
;; @009f                               v35 = iadd.i64 v17, v34
;; @009f                               v36 = iadd.i64 v29, v34
;; @009f                               v37 = ireduce.i32 v30
;; @009f                               v38 = iadd.i32 v4, v37
;; @009f                               brif v33, block3(v17, v29, v4), block4(v35, v36, v38)
;;
;;                                 block3(v39: i64, v40: i64, v41: i32):
;; @009f                               v101 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v42 = load.i64 notrap aligned v101+8
;; @009f                               v43 = ireduce.i32 v42
;; @009f                               v44 = icmp uge v41, v43
;; @009f                               v45 = uextend.i64 v41
;; @009f                               v99 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v46 = load.i64 notrap aligned v99
;;                                     v98 = iconst.i64 3
;; @009f                               v47 = ishl v45, v98  ; v98 = 3
;; @009f                               v48 = iadd v46, v47
;; @009f                               v49 = iconst.i64 0
;; @009f                               v50 = select_spectre_guard v44, v49, v48  ; v49 = 0
;; @009f                               v51 = load.i64 user6 aligned table v50
;;                                     v97 = iconst.i64 -2
;; @009f                               v52 = band v51, v97  ; v97 = -2
;; @009f                               brif v51, block7(v52), block6
;;
;;                                 block4(v63: i64, v64: i64, v65: i32):
;; @009f                               v66 = isub v63, v31  ; v31 = 8
;; @009f                               v67 = isub v64, v31  ; v31 = 8
;; @009f                               v68 = iconst.i32 1
;; @009f                               v69 = isub v65, v68  ; v68 = 1
;; @009f                               v95 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v70 = load.i64 notrap aligned v95+8
;; @009f                               v71 = ireduce.i32 v70
;; @009f                               v72 = icmp uge v69, v71
;; @009f                               v73 = uextend.i64 v69
;; @009f                               v93 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v74 = load.i64 notrap aligned v93
;;                                     v92 = iconst.i64 3
;; @009f                               v75 = ishl v73, v92  ; v92 = 3
;; @009f                               v76 = iadd v74, v75
;; @009f                               v77 = iconst.i64 0
;; @009f                               v78 = select_spectre_guard v72, v77, v76  ; v77 = 0
;; @009f                               v79 = load.i64 user6 aligned table v78
;;                                     v91 = iconst.i64 -2
;; @009f                               v80 = band v79, v91  ; v91 = -2
;; @009f                               brif v79, block9(v80), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v54 = iconst.i32 0
;; @009f                               v56 = uextend.i64 v41
;; @009f                               v57 = call fn0(v0, v54, v56)  ; v54 = 0
;; @009f                               jump block7(v57)
;;
;;                                 block7(v53: i64):
;;                                     v90 = iconst.i64 1
;; @009f                               v58 = bor v53, v90  ; v90 = 1
;; @009f                               store notrap aligned v58, v39
;; @009f                               v59 = iadd.i64 v39, v31  ; v31 = 8
;; @009f                               v60 = iadd.i64 v40, v31  ; v31 = 8
;;                                     v89 = iconst.i32 1
;; @009f                               v61 = iadd.i32 v41, v89  ; v89 = 1
;; @009f                               v62 = icmp eq v60, v36
;; @009f                               brif v62, block5, block3(v59, v60, v61)
;;
;;                                 block8 cold:
;; @009f                               v82 = iconst.i32 0
;; @009f                               v84 = uextend.i64 v69
;; @009f                               v85 = call fn0(v0, v82, v84)  ; v82 = 0
;; @009f                               jump block9(v85)
;;
;;                                 block9(v81: i64):
;;                                     v88 = iconst.i64 1
;; @009f                               v86 = bor v81, v88  ; v88 = 1
;; @009f                               store notrap aligned v86, v66
;; @009f                               v87 = icmp.i64 eq v67, v29
;; @009f                               brif v87, block5, block4(v66, v67, v69)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
