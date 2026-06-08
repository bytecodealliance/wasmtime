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
;;     region0 = 1073741824 "PublicTable"
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
;; @0090                               v109 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v7 = load.i64 notrap aligned v109+8
;; @0090                               v8 = ireduce.i32 v7
;; @0090                               v9 = uextend.i64 v8
;; @0090                               v10 = uextend.i64 v3
;; @0090                               v11 = uextend.i64 v5
;; @0090                               v12 = iconst.i64 1
;; @0090                               v13 = imul v11, v12  ; v12 = 1
;; @0090                               v14 = iadd v10, v13
;; @0090                               v15 = icmp ugt v14, v9
;; @0090                               trapnz v15, user6
;; @0090                               v107 = load.i64 notrap aligned readonly can_move v0+48
;; @0090                               v16 = load.i64 notrap aligned v107
;; @0090                               v17 = uextend.i64 v3
;; @0090                               v18 = iconst.i64 8
;; @0090                               v19 = imul v17, v18  ; v18 = 8
;; @0090                               v20 = iadd v16, v19
;; @0090                               v21 = iconst.i32 6
;; @0090                               v22 = uextend.i64 v21  ; v21 = 6
;; @0090                               v23 = uextend.i64 v4
;; @0090                               v24 = uextend.i64 v5
;; @0090                               v25 = iconst.i64 1
;; @0090                               v26 = imul v24, v25  ; v25 = 1
;; @0090                               v27 = iadd v23, v26
;; @0090                               v28 = icmp ugt v27, v22
;; @0090                               trapnz v28, user6
;; @0090                               v29 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v30 = uextend.i64 v4
;; @0090                               v31 = iconst.i64 8
;; @0090                               v32 = imul v30, v31  ; v31 = 8
;; @0090                               v33 = iadd v29, v32
;; @0090                               v34 = uextend.i64 v5
;; @0090                               v35 = iconst.i64 8
;; @0090                               v36 = imul v34, v35  ; v35 = 8
;; @0090                               v37 = iconst.i64 8
;; @0090                               v38 = imul v34, v37  ; v37 = 8
;; @0090                               brif v34, block2, block5
;;
;;                                 block2:
;; @0090                               v39 = icmp.i64 ult v20, v33
;; @0090                               v40 = iconst.i64 8
;; @0090                               v41 = imul.i64 v34, v40  ; v40 = 8
;; @0090                               v42 = iconst.i64 8
;; @0090                               v43 = imul.i64 v34, v42  ; v42 = 8
;; @0090                               v44 = iadd.i64 v20, v41
;; @0090                               v45 = iadd.i64 v33, v43
;; @0090                               v46 = ireduce.i32 v34
;; @0090                               v47 = iadd.i32 v4, v46
;; @0090                               brif v39, block3(v20, v33, v4), block4(v44, v45, v47)
;;
;;                                 block3(v48: i64, v49: i64, v50: i32):
;; @0090                               v51 = iconst.i32 6
;; @0090                               v52 = icmp uge v50, v51  ; v51 = 6
;; @0090                               v53 = uextend.i64 v50
;; @0090                               v54 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v55 = iconst.i64 3
;; @0090                               v56 = ishl v53, v55  ; v55 = 3
;; @0090                               v57 = iadd v54, v56
;; @0090                               v58 = iconst.i64 0
;; @0090                               v59 = select_spectre_guard v52, v58, v57  ; v58 = 0
;; @0090                               v60 = load.i64 user6 aligned region0 v59
;; @0090                               v61 = iconst.i64 -2
;; @0090                               v62 = band v60, v61  ; v61 = -2
;; @0090                               brif v60, block7(v62), block6
;;
;;                                 block4(v76: i64, v77: i64, v78: i32):
;; @0090                               v79 = iconst.i64 8
;; @0090                               v80 = isub v76, v79  ; v79 = 8
;; @0090                               v81 = iconst.i64 8
;; @0090                               v82 = isub v77, v81  ; v81 = 8
;; @0090                               v83 = iconst.i32 1
;; @0090                               v84 = isub v78, v83  ; v83 = 1
;; @0090                               v85 = iconst.i32 6
;; @0090                               v86 = icmp uge v84, v85  ; v85 = 6
;; @0090                               v87 = uextend.i64 v84
;; @0090                               v88 = load.i64 notrap aligned readonly can_move v0+72
;; @0090                               v89 = iconst.i64 3
;; @0090                               v90 = ishl v87, v89  ; v89 = 3
;; @0090                               v91 = iadd v88, v90
;; @0090                               v92 = iconst.i64 0
;; @0090                               v93 = select_spectre_guard v86, v92, v91  ; v92 = 0
;; @0090                               v94 = load.i64 user6 aligned region0 v93
;; @0090                               v95 = iconst.i64 -2
;; @0090                               v96 = band v94, v95  ; v95 = -2
;; @0090                               brif v94, block9(v96), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v64 = iconst.i32 1
;; @0090                               v65 = uextend.i64 v50
;; @0090                               v66 = call fn0(v0, v64, v65)  ; v64 = 1
;; @0090                               jump block7(v66)
;;
;;                                 block7(v63: i64):
;; @0090                               v67 = iconst.i64 1
;; @0090                               v68 = bor v63, v67  ; v67 = 1
;; @0090                               store notrap aligned v68, v48
;; @0090                               v69 = iconst.i64 8
;; @0090                               v70 = iadd.i64 v48, v69  ; v69 = 8
;; @0090                               v71 = iconst.i64 8
;; @0090                               v72 = iadd.i64 v49, v71  ; v71 = 8
;; @0090                               v73 = iconst.i32 1
;; @0090                               v74 = iadd.i32 v50, v73  ; v73 = 1
;; @0090                               v75 = icmp eq v72, v45
;; @0090                               brif v75, block5, block3(v70, v72, v74)
;;
;;                                 block8 cold:
;; @0090                               v98 = iconst.i32 1
;; @0090                               v99 = uextend.i64 v84
;; @0090                               v100 = call fn0(v0, v98, v99)  ; v98 = 1
;; @0090                               jump block9(v100)
;;
;;                                 block9(v97: i64):
;; @0090                               v101 = iconst.i64 1
;; @0090                               v102 = bor v97, v101  ; v101 = 1
;; @0090                               store notrap aligned v102, v80
;; @0090                               v103 = icmp.i64 eq v82, v33
;; @0090                               brif v103, block5, block4(v80, v82, v84)
;;
;;                                 block1:
;; @0094                               return v2
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 1073741824 "PublicTable"
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
;; @009f                               v11 = iconst.i64 1
;; @009f                               v12 = imul v10, v11  ; v11 = 1
;; @009f                               v13 = iadd v9, v12
;; @009f                               v14 = icmp ugt v13, v8
;; @009f                               trapnz v14, user6
;; @009f                               v15 = load.i64 notrap aligned readonly can_move v0+72
;; @009f                               v16 = uextend.i64 v3
;; @009f                               v17 = iconst.i64 8
;; @009f                               v18 = imul v16, v17  ; v17 = 8
;; @009f                               v19 = iadd v15, v18
;; @009f                               v116 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v20 = load.i64 notrap aligned v116+8
;; @009f                               v21 = ireduce.i32 v20
;; @009f                               v22 = uextend.i64 v21
;; @009f                               v23 = uextend.i64 v4
;; @009f                               v24 = uextend.i64 v5
;; @009f                               v25 = iconst.i64 1
;; @009f                               v26 = imul v24, v25  ; v25 = 1
;; @009f                               v27 = iadd v23, v26
;; @009f                               v28 = icmp ugt v27, v22
;; @009f                               trapnz v28, user6
;; @009f                               v114 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v29 = load.i64 notrap aligned v114
;; @009f                               v30 = uextend.i64 v4
;; @009f                               v31 = iconst.i64 8
;; @009f                               v32 = imul v30, v31  ; v31 = 8
;; @009f                               v33 = iadd v29, v32
;; @009f                               v34 = uextend.i64 v5
;; @009f                               v35 = iconst.i64 8
;; @009f                               v36 = imul v34, v35  ; v35 = 8
;; @009f                               v37 = iconst.i64 8
;; @009f                               v38 = imul v34, v37  ; v37 = 8
;; @009f                               brif v34, block2, block5
;;
;;                                 block2:
;; @009f                               v39 = icmp.i64 ult v19, v33
;; @009f                               v40 = iconst.i64 8
;; @009f                               v41 = imul.i64 v34, v40  ; v40 = 8
;; @009f                               v42 = iconst.i64 8
;; @009f                               v43 = imul.i64 v34, v42  ; v42 = 8
;; @009f                               v44 = iadd.i64 v19, v41
;; @009f                               v45 = iadd.i64 v33, v43
;; @009f                               v46 = ireduce.i32 v34
;; @009f                               v47 = iadd.i32 v4, v46
;; @009f                               brif v39, block3(v19, v33, v4), block4(v44, v45, v47)
;;
;;                                 block3(v48: i64, v49: i64, v50: i32):
;; @009f                               v112 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v51 = load.i64 notrap aligned v112+8
;; @009f                               v52 = ireduce.i32 v51
;; @009f                               v53 = icmp uge v50, v52
;; @009f                               v54 = uextend.i64 v50
;; @009f                               v110 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v55 = load.i64 notrap aligned v110
;; @009f                               v56 = iconst.i64 3
;; @009f                               v57 = ishl v54, v56  ; v56 = 3
;; @009f                               v58 = iadd v55, v57
;; @009f                               v59 = iconst.i64 0
;; @009f                               v60 = select_spectre_guard v53, v59, v58  ; v59 = 0
;; @009f                               v61 = load.i64 user6 aligned region0 v60
;; @009f                               v62 = iconst.i64 -2
;; @009f                               v63 = band v61, v62  ; v62 = -2
;; @009f                               brif v61, block7(v63), block6
;;
;;                                 block4(v77: i64, v78: i64, v79: i32):
;; @009f                               v80 = iconst.i64 8
;; @009f                               v81 = isub v77, v80  ; v80 = 8
;; @009f                               v82 = iconst.i64 8
;; @009f                               v83 = isub v78, v82  ; v82 = 8
;; @009f                               v84 = iconst.i32 1
;; @009f                               v85 = isub v79, v84  ; v84 = 1
;; @009f                               v108 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v86 = load.i64 notrap aligned v108+8
;; @009f                               v87 = ireduce.i32 v86
;; @009f                               v88 = icmp uge v85, v87
;; @009f                               v89 = uextend.i64 v85
;; @009f                               v106 = load.i64 notrap aligned readonly can_move v0+48
;; @009f                               v90 = load.i64 notrap aligned v106
;; @009f                               v91 = iconst.i64 3
;; @009f                               v92 = ishl v89, v91  ; v91 = 3
;; @009f                               v93 = iadd v90, v92
;; @009f                               v94 = iconst.i64 0
;; @009f                               v95 = select_spectre_guard v88, v94, v93  ; v94 = 0
;; @009f                               v96 = load.i64 user6 aligned region0 v95
;; @009f                               v97 = iconst.i64 -2
;; @009f                               v98 = band v96, v97  ; v97 = -2
;; @009f                               brif v96, block9(v98), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v65 = iconst.i32 0
;; @009f                               v66 = uextend.i64 v50
;; @009f                               v67 = call fn0(v0, v65, v66)  ; v65 = 0
;; @009f                               jump block7(v67)
;;
;;                                 block7(v64: i64):
;; @009f                               v68 = iconst.i64 1
;; @009f                               v69 = bor v64, v68  ; v68 = 1
;; @009f                               store notrap aligned v69, v48
;; @009f                               v70 = iconst.i64 8
;; @009f                               v71 = iadd.i64 v48, v70  ; v70 = 8
;; @009f                               v72 = iconst.i64 8
;; @009f                               v73 = iadd.i64 v49, v72  ; v72 = 8
;; @009f                               v74 = iconst.i32 1
;; @009f                               v75 = iadd.i32 v50, v74  ; v74 = 1
;; @009f                               v76 = icmp eq v73, v45
;; @009f                               brif v76, block5, block3(v71, v73, v75)
;;
;;                                 block8 cold:
;; @009f                               v100 = iconst.i32 0
;; @009f                               v101 = uextend.i64 v85
;; @009f                               v102 = call fn0(v0, v100, v101)  ; v100 = 0
;; @009f                               jump block9(v102)
;;
;;                                 block9(v99: i64):
;; @009f                               v103 = iconst.i64 1
;; @009f                               v104 = bor v99, v103  ; v103 = 1
;; @009f                               store notrap aligned v104, v81
;; @009f                               v105 = icmp.i64 eq v83, v33
;; @009f                               brif v105, block5, block4(v81, v83, v85)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
