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
;; @0090                               v97 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v7 = load.i64 notrap aligned v97+8
;; @0090                               v8 = ireduce.i32 v7
;; @0090                               v9 = uextend.i64 v8
;; @0090                               v10 = uextend.i64 v3
;; @0090                               v11 = uextend.i64 v5
;; @0090                               v12 = iadd v10, v11
;; @0090                               v13 = icmp ugt v12, v9
;; @0090                               trapnz v13, user6
;; @0090                               v95 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v14 = load.i64 notrap aligned v95
;; @0090                               v15 = uextend.i64 v3
;;                                     v94 = iconst.i64 8
;; @0090                               v16 = imul v15, v94  ; v94 = 8
;; @0090                               v17 = iadd v14, v16
;; @0090                               v18 = iconst.i32 6
;; @0090                               v19 = uextend.i64 v18  ; v18 = 6
;; @0090                               v20 = uextend.i64 v4
;; @0090                               v21 = uextend.i64 v5
;; @0090                               v22 = iadd v20, v21
;; @0090                               v23 = icmp ugt v22, v19
;; @0090                               trapnz v23, user6
;; @0090                               v24 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v25 = uextend.i64 v4
;;                                     v92 = iconst.i64 8
;; @0090                               v26 = imul v25, v92  ; v92 = 8
;; @0090                               v27 = iadd v24, v26
;; @0090                               v28 = uextend.i64 v5
;; @0090                               v29 = iconst.i64 8
;; @0090                               brif v28, block2, block5
;;
;;                                 block2:
;; @0090                               v30 = icmp.i64 ult v17, v27
;; @0090                               v31 = imul.i64 v28, v29  ; v29 = 8
;; @0090                               v32 = iadd.i64 v17, v31
;; @0090                               v33 = iadd.i64 v27, v31
;; @0090                               v34 = ireduce.i32 v28
;; @0090                               v35 = iadd.i32 v4, v34
;; @0090                               brif v30, block3(v17, v27, v4), block4(v32, v33, v35)
;;
;;                                 block3(v36: i64, v37: i64, v38: i32):
;; @0090                               v39 = iconst.i32 6
;; @0090                               v40 = icmp uge v38, v39  ; v39 = 6
;; @0090                               v41 = uextend.i64 v38
;; @0090                               v42 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v90 = iconst.i64 3
;; @0090                               v43 = ishl v41, v90  ; v90 = 3
;; @0090                               v44 = iadd v42, v43
;; @0090                               v45 = iconst.i64 0
;; @0090                               v46 = select_spectre_guard v40, v45, v44  ; v45 = 0
;; @0090                               v47 = load.i64 user6 aligned table v46
;;                                     v89 = iconst.i64 -2
;; @0090                               v48 = band v47, v89  ; v89 = -2
;; @0090                               brif v47, block7(v48), block6
;;
;;                                 block4(v59: i64, v60: i64, v61: i32):
;; @0090                               v62 = isub v59, v29  ; v29 = 8
;; @0090                               v63 = isub v60, v29  ; v29 = 8
;; @0090                               v64 = iconst.i32 1
;; @0090                               v65 = isub v61, v64  ; v64 = 1
;; @0090                               v66 = iconst.i32 6
;; @0090                               v67 = icmp uge v65, v66  ; v66 = 6
;; @0090                               v68 = uextend.i64 v65
;; @0090                               v69 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v87 = iconst.i64 3
;; @0090                               v70 = ishl v68, v87  ; v87 = 3
;; @0090                               v71 = iadd v69, v70
;; @0090                               v72 = iconst.i64 0
;; @0090                               v73 = select_spectre_guard v67, v72, v71  ; v72 = 0
;; @0090                               v74 = load.i64 user6 aligned table v73
;;                                     v86 = iconst.i64 -2
;; @0090                               v75 = band v74, v86  ; v86 = -2
;; @0090                               brif v74, block9(v75), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v50 = iconst.i32 1
;; @0090                               v52 = uextend.i64 v38
;; @0090                               v53 = call fn0(v0, v50, v52)  ; v50 = 1
;; @0090                               jump block7(v53)
;;
;;                                 block7(v49: i64):
;;                                     v85 = iconst.i64 1
;; @0090                               v54 = bor v49, v85  ; v85 = 1
;; @0090                               store notrap aligned v54, v36
;; @0090                               v55 = iadd.i64 v36, v29  ; v29 = 8
;; @0090                               v56 = iadd.i64 v37, v29  ; v29 = 8
;;                                     v84 = iconst.i32 1
;; @0090                               v57 = iadd.i32 v38, v84  ; v84 = 1
;; @0090                               v58 = icmp eq v56, v33
;; @0090                               brif v58, block5, block3(v55, v56, v57)
;;
;;                                 block8 cold:
;; @0090                               v77 = iconst.i32 1
;; @0090                               v79 = uextend.i64 v65
;; @0090                               v80 = call fn0(v0, v77, v79)  ; v77 = 1
;; @0090                               jump block9(v80)
;;
;;                                 block9(v76: i64):
;;                                     v83 = iconst.i64 1
;; @0090                               v81 = bor v76, v83  ; v83 = 1
;; @0090                               store notrap aligned v81, v62
;; @0090                               v82 = icmp.i64 eq v63, v27
;; @0090                               brif v82, block5, block4(v62, v63, v65)
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
;; @009f                               v11 = iadd v9, v10
;; @009f                               v12 = icmp ugt v11, v8
;; @009f                               trapnz v12, user6
;; @009f                               v13 = load.i64 notrap aligned readonly can_move v0+72
;; @009f                               v14 = uextend.i64 v3
;;                                     v105 = iconst.i64 8
;; @009f                               v15 = imul v14, v105  ; v105 = 8
;; @009f                               v16 = iadd v13, v15
;; @009f                               v103 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v17 = load.i64 notrap aligned v103+8
;; @009f                               v18 = ireduce.i32 v17
;; @009f                               v19 = uextend.i64 v18
;; @009f                               v20 = uextend.i64 v4
;; @009f                               v21 = uextend.i64 v5
;; @009f                               v22 = iadd v20, v21
;; @009f                               v23 = icmp ugt v22, v19
;; @009f                               trapnz v23, user6
;; @009f                               v101 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v24 = load.i64 notrap aligned v101
;; @009f                               v25 = uextend.i64 v4
;;                                     v100 = iconst.i64 8
;; @009f                               v26 = imul v25, v100  ; v100 = 8
;; @009f                               v27 = iadd v24, v26
;; @009f                               v28 = uextend.i64 v5
;; @009f                               v29 = iconst.i64 8
;; @009f                               brif v28, block2, block5
;;
;;                                 block2:
;; @009f                               v30 = icmp.i64 ult v16, v27
;; @009f                               v31 = imul.i64 v28, v29  ; v29 = 8
;; @009f                               v32 = iadd.i64 v16, v31
;; @009f                               v33 = iadd.i64 v27, v31
;; @009f                               v34 = ireduce.i32 v28
;; @009f                               v35 = iadd.i32 v4, v34
;; @009f                               brif v30, block3(v16, v27, v4), block4(v32, v33, v35)
;;
;;                                 block3(v36: i64, v37: i64, v38: i32):
;; @009f                               v98 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v39 = load.i64 notrap aligned v98+8
;; @009f                               v40 = ireduce.i32 v39
;; @009f                               v41 = icmp uge v38, v40
;; @009f                               v42 = uextend.i64 v38
;; @009f                               v96 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v43 = load.i64 notrap aligned v96
;;                                     v95 = iconst.i64 3
;; @009f                               v44 = ishl v42, v95  ; v95 = 3
;; @009f                               v45 = iadd v43, v44
;; @009f                               v46 = iconst.i64 0
;; @009f                               v47 = select_spectre_guard v41, v46, v45  ; v46 = 0
;; @009f                               v48 = load.i64 user6 aligned table v47
;;                                     v94 = iconst.i64 -2
;; @009f                               v49 = band v48, v94  ; v94 = -2
;; @009f                               brif v48, block7(v49), block6
;;
;;                                 block4(v60: i64, v61: i64, v62: i32):
;; @009f                               v63 = isub v60, v29  ; v29 = 8
;; @009f                               v64 = isub v61, v29  ; v29 = 8
;; @009f                               v65 = iconst.i32 1
;; @009f                               v66 = isub v62, v65  ; v65 = 1
;; @009f                               v92 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v67 = load.i64 notrap aligned v92+8
;; @009f                               v68 = ireduce.i32 v67
;; @009f                               v69 = icmp uge v66, v68
;; @009f                               v70 = uextend.i64 v66
;; @009f                               v90 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v71 = load.i64 notrap aligned v90
;;                                     v89 = iconst.i64 3
;; @009f                               v72 = ishl v70, v89  ; v89 = 3
;; @009f                               v73 = iadd v71, v72
;; @009f                               v74 = iconst.i64 0
;; @009f                               v75 = select_spectre_guard v69, v74, v73  ; v74 = 0
;; @009f                               v76 = load.i64 user6 aligned table v75
;;                                     v88 = iconst.i64 -2
;; @009f                               v77 = band v76, v88  ; v88 = -2
;; @009f                               brif v76, block9(v77), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v51 = iconst.i32 0
;; @009f                               v53 = uextend.i64 v38
;; @009f                               v54 = call fn0(v0, v51, v53)  ; v51 = 0
;; @009f                               jump block7(v54)
;;
;;                                 block7(v50: i64):
;;                                     v87 = iconst.i64 1
;; @009f                               v55 = bor v50, v87  ; v87 = 1
;; @009f                               store notrap aligned v55, v36
;; @009f                               v56 = iadd.i64 v36, v29  ; v29 = 8
;; @009f                               v57 = iadd.i64 v37, v29  ; v29 = 8
;;                                     v86 = iconst.i32 1
;; @009f                               v58 = iadd.i32 v38, v86  ; v86 = 1
;; @009f                               v59 = icmp eq v57, v33
;; @009f                               brif v59, block5, block3(v56, v57, v58)
;;
;;                                 block8 cold:
;; @009f                               v79 = iconst.i32 0
;; @009f                               v81 = uextend.i64 v66
;; @009f                               v82 = call fn0(v0, v79, v81)  ; v79 = 0
;; @009f                               jump block9(v82)
;;
;;                                 block9(v78: i64):
;;                                     v85 = iconst.i64 1
;; @009f                               v83 = bor v78, v85  ; v85 = 1
;; @009f                               store notrap aligned v83, v63
;; @009f                               v84 = icmp.i64 eq v64, v27
;; @009f                               brif v84, block5, block4(v63, v64, v66)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
