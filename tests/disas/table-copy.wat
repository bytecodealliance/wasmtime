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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 48 "VMContext+0x30"
;;     region3 = 2684354560 "VMTableDefinition+0x0"
;;     region4 = 2684354568 "VMTableDefinition+0x8"
;;     region5 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0090                               v7 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0090                               v8 = load.i64 notrap aligned region4 v7+8
;; @0090                               v9 = ireduce.i32 v8
;; @0090                               v10 = uextend.i64 v9
;; @0090                               v11 = uextend.i64 v3
;; @0090                               v12 = uextend.i64 v5
;; @0090                               v13 = iconst.i64 1
;; @0090                               v14 = imul v12, v13  ; v13 = 1
;; @0090                               v15 = iadd v11, v14
;; @0090                               v16 = icmp ugt v15, v10
;; @0090                               trapnz v16, user6
;; @0090                               v17 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0090                               v18 = load.i64 notrap aligned region3 v17
;; @0090                               v19 = uextend.i64 v3
;; @0090                               v20 = iconst.i64 8
;; @0090                               v21 = imul v19, v20  ; v20 = 8
;; @0090                               v22 = iadd v18, v21
;; @0090                               v23 = iconst.i32 6
;; @0090                               v24 = uextend.i64 v23  ; v23 = 6
;; @0090                               v25 = uextend.i64 v4
;; @0090                               v26 = uextend.i64 v5
;; @0090                               v27 = iconst.i64 1
;; @0090                               v28 = imul v26, v27  ; v27 = 1
;; @0090                               v29 = iadd v25, v28
;; @0090                               v30 = icmp ugt v29, v24
;; @0090                               trapnz v30, user6
;; @0090                               v31 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v32 = uextend.i64 v4
;; @0090                               v33 = iconst.i64 8
;; @0090                               v34 = imul v32, v33  ; v33 = 8
;; @0090                               v35 = iadd v31, v34
;; @0090                               v36 = uextend.i64 v5
;; @0090                               v37 = iconst.i64 8
;; @0090                               v38 = imul v36, v37  ; v37 = 8
;; @0090                               v39 = iconst.i64 8
;; @0090                               v40 = imul v36, v39  ; v39 = 8
;; @0090                               brif v36, block2, block5
;;
;;                                 block2:
;; @0090                               v41 = icmp.i64 ult v22, v35
;; @0090                               v42 = iconst.i64 8
;; @0090                               v43 = imul.i64 v36, v42  ; v42 = 8
;; @0090                               v44 = iconst.i64 8
;; @0090                               v45 = imul.i64 v36, v44  ; v44 = 8
;; @0090                               v46 = iadd.i64 v22, v43
;; @0090                               v47 = iadd.i64 v35, v45
;; @0090                               v48 = ireduce.i32 v36
;; @0090                               v49 = iadd.i32 v4, v48
;; @0090                               brif v41, block3(v22, v35, v4), block4(v46, v47, v49)
;;
;;                                 block3(v50: i64, v51: i64, v52: i32):
;; @0090                               v53 = iconst.i32 6
;; @0090                               v54 = icmp uge v52, v53  ; v53 = 6
;; @0090                               v55 = uextend.i64 v52
;; @0090                               v56 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v57 = iconst.i64 3
;; @0090                               v58 = ishl v55, v57  ; v57 = 3
;; @0090                               v59 = iadd v56, v58
;; @0090                               v60 = iconst.i64 0
;; @0090                               v61 = select_spectre_guard v54, v60, v59  ; v60 = 0
;; @0090                               v62 = load.i64 user6 aligned region5 v61
;; @0090                               v63 = iconst.i64 -2
;; @0090                               v64 = band v62, v63  ; v63 = -2
;; @0090                               brif v62, block7(v64), block6
;;
;;                                 block4(v78: i64, v79: i64, v80: i32):
;; @0090                               v81 = iconst.i64 8
;; @0090                               v82 = isub v78, v81  ; v81 = 8
;; @0090                               v83 = iconst.i64 8
;; @0090                               v84 = isub v79, v83  ; v83 = 8
;; @0090                               v85 = iconst.i32 1
;; @0090                               v86 = isub v80, v85  ; v85 = 1
;; @0090                               v87 = iconst.i32 6
;; @0090                               v88 = icmp uge v86, v87  ; v87 = 6
;; @0090                               v89 = uextend.i64 v86
;; @0090                               v90 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v91 = iconst.i64 3
;; @0090                               v92 = ishl v89, v91  ; v91 = 3
;; @0090                               v93 = iadd v90, v92
;; @0090                               v94 = iconst.i64 0
;; @0090                               v95 = select_spectre_guard v88, v94, v93  ; v94 = 0
;; @0090                               v96 = load.i64 user6 aligned region5 v95
;; @0090                               v97 = iconst.i64 -2
;; @0090                               v98 = band v96, v97  ; v97 = -2
;; @0090                               brif v96, block9(v98), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v66 = iconst.i32 1
;; @0090                               v67 = uextend.i64 v52
;; @0090                               v68 = call fn0(v0, v66, v67)  ; v66 = 1
;; @0090                               jump block7(v68)
;;
;;                                 block7(v65: i64):
;; @0090                               v69 = iconst.i64 1
;; @0090                               v70 = bor v65, v69  ; v69 = 1
;; @0090                               store notrap aligned region5 v70, v50
;; @0090                               v71 = iconst.i64 8
;; @0090                               v72 = iadd.i64 v50, v71  ; v71 = 8
;; @0090                               v73 = iconst.i64 8
;; @0090                               v74 = iadd.i64 v51, v73  ; v73 = 8
;; @0090                               v75 = iconst.i32 1
;; @0090                               v76 = iadd.i32 v52, v75  ; v75 = 1
;; @0090                               v77 = icmp eq v74, v47
;; @0090                               brif v77, block5, block3(v72, v74, v76)
;;
;;                                 block8 cold:
;; @0090                               v100 = iconst.i32 1
;; @0090                               v101 = uextend.i64 v86
;; @0090                               v102 = call fn0(v0, v100, v101)  ; v100 = 1
;; @0090                               jump block9(v102)
;;
;;                                 block9(v99: i64):
;; @0090                               v103 = iconst.i64 1
;; @0090                               v104 = bor v99, v103  ; v103 = 1
;; @0090                               store notrap aligned region5 v104, v82
;; @0090                               v105 = icmp.i64 eq v84, v35
;; @0090                               brif v105, block5, block4(v82, v84, v86)
;;
;;                                 block1:
;; @0094                               return v2
;; }
;;
;; function u0:4(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 48 "VMContext+0x30"
;;     region4 = 2684354568 "VMTableDefinition+0x8"
;;     region5 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
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
;; @009f                               v15 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @009f                               v16 = uextend.i64 v3
;; @009f                               v17 = iconst.i64 8
;; @009f                               v18 = imul v16, v17  ; v17 = 8
;; @009f                               v19 = iadd v15, v18
;; @009f                               v20 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v21 = load.i64 notrap aligned region4 v20+8
;; @009f                               v22 = ireduce.i32 v21
;; @009f                               v23 = uextend.i64 v22
;; @009f                               v24 = uextend.i64 v4
;; @009f                               v25 = uextend.i64 v5
;; @009f                               v26 = iconst.i64 1
;; @009f                               v27 = imul v25, v26  ; v26 = 1
;; @009f                               v28 = iadd v24, v27
;; @009f                               v29 = icmp ugt v28, v23
;; @009f                               trapnz v29, user6
;; @009f                               v30 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v31 = load.i64 notrap aligned region2 v30
;; @009f                               v32 = uextend.i64 v4
;; @009f                               v33 = iconst.i64 8
;; @009f                               v34 = imul v32, v33  ; v33 = 8
;; @009f                               v35 = iadd v31, v34
;; @009f                               v36 = uextend.i64 v5
;; @009f                               v37 = iconst.i64 8
;; @009f                               v38 = imul v36, v37  ; v37 = 8
;; @009f                               v39 = iconst.i64 8
;; @009f                               v40 = imul v36, v39  ; v39 = 8
;; @009f                               brif v36, block2, block5
;;
;;                                 block2:
;; @009f                               v41 = icmp.i64 ult v19, v35
;; @009f                               v42 = iconst.i64 8
;; @009f                               v43 = imul.i64 v36, v42  ; v42 = 8
;; @009f                               v44 = iconst.i64 8
;; @009f                               v45 = imul.i64 v36, v44  ; v44 = 8
;; @009f                               v46 = iadd.i64 v19, v43
;; @009f                               v47 = iadd.i64 v35, v45
;; @009f                               v48 = ireduce.i32 v36
;; @009f                               v49 = iadd.i32 v4, v48
;; @009f                               brif v41, block3(v19, v35, v4), block4(v46, v47, v49)
;;
;;                                 block3(v50: i64, v51: i64, v52: i32):
;; @009f                               v53 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v54 = load.i64 notrap aligned region4 v53+8
;; @009f                               v55 = ireduce.i32 v54
;; @009f                               v56 = icmp uge v52, v55
;; @009f                               v57 = uextend.i64 v52
;; @009f                               v58 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v59 = load.i64 notrap aligned region2 v58
;; @009f                               v60 = iconst.i64 3
;; @009f                               v61 = ishl v57, v60  ; v60 = 3
;; @009f                               v62 = iadd v59, v61
;; @009f                               v63 = iconst.i64 0
;; @009f                               v64 = select_spectre_guard v56, v63, v62  ; v63 = 0
;; @009f                               v65 = load.i64 user6 aligned region5 v64
;; @009f                               v66 = iconst.i64 -2
;; @009f                               v67 = band v65, v66  ; v66 = -2
;; @009f                               brif v65, block7(v67), block6
;;
;;                                 block4(v81: i64, v82: i64, v83: i32):
;; @009f                               v84 = iconst.i64 8
;; @009f                               v85 = isub v81, v84  ; v84 = 8
;; @009f                               v86 = iconst.i64 8
;; @009f                               v87 = isub v82, v86  ; v86 = 8
;; @009f                               v88 = iconst.i32 1
;; @009f                               v89 = isub v83, v88  ; v88 = 1
;; @009f                               v90 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v91 = load.i64 notrap aligned region4 v90+8
;; @009f                               v92 = ireduce.i32 v91
;; @009f                               v93 = icmp uge v89, v92
;; @009f                               v94 = uextend.i64 v89
;; @009f                               v95 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v96 = load.i64 notrap aligned region2 v95
;; @009f                               v97 = iconst.i64 3
;; @009f                               v98 = ishl v94, v97  ; v97 = 3
;; @009f                               v99 = iadd v96, v98
;; @009f                               v100 = iconst.i64 0
;; @009f                               v101 = select_spectre_guard v93, v100, v99  ; v100 = 0
;; @009f                               v102 = load.i64 user6 aligned region5 v101
;; @009f                               v103 = iconst.i64 -2
;; @009f                               v104 = band v102, v103  ; v103 = -2
;; @009f                               brif v102, block9(v104), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v69 = iconst.i32 0
;; @009f                               v70 = uextend.i64 v52
;; @009f                               v71 = call fn0(v0, v69, v70)  ; v69 = 0
;; @009f                               jump block7(v71)
;;
;;                                 block7(v68: i64):
;; @009f                               v72 = iconst.i64 1
;; @009f                               v73 = bor v68, v72  ; v72 = 1
;; @009f                               store notrap aligned region5 v73, v50
;; @009f                               v74 = iconst.i64 8
;; @009f                               v75 = iadd.i64 v50, v74  ; v74 = 8
;; @009f                               v76 = iconst.i64 8
;; @009f                               v77 = iadd.i64 v51, v76  ; v76 = 8
;; @009f                               v78 = iconst.i32 1
;; @009f                               v79 = iadd.i32 v52, v78  ; v78 = 1
;; @009f                               v80 = icmp eq v77, v47
;; @009f                               brif v80, block5, block3(v75, v77, v79)
;;
;;                                 block8 cold:
;; @009f                               v106 = iconst.i32 0
;; @009f                               v107 = uextend.i64 v89
;; @009f                               v108 = call fn0(v0, v106, v107)  ; v106 = 0
;; @009f                               jump block9(v108)
;;
;;                                 block9(v105: i64):
;; @009f                               v109 = iconst.i64 1
;; @009f                               v110 = bor v105, v109  ; v109 = 1
;; @009f                               store notrap aligned region5 v110, v85
;; @009f                               v111 = icmp.i64 eq v87, v35
;; @009f                               brif v111, block5, block4(v85, v87, v89)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
