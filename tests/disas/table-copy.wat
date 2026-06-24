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
;; @0090                               v6 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0090                               v7 = load.i64 notrap aligned region4 v6+8
;; @0090                               v8 = ireduce.i32 v7
;; @0090                               v9 = uextend.i64 v8
;; @0090                               v10 = uextend.i64 v3
;; @0090                               v11 = uextend.i64 v5
;; @0090                               v12 = iconst.i64 1
;; @0090                               v13 = imul v11, v12  ; v12 = 1
;; @0090                               v14 = iadd v10, v13
;; @0090                               v15 = icmp ugt v14, v9
;; @0090                               trapnz v15, user6
;; @0090                               v16 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0090                               v17 = load.i64 notrap aligned region3 v16
;; @0090                               v18 = uextend.i64 v3
;; @0090                               v19 = iconst.i64 8
;; @0090                               v20 = imul v18, v19  ; v19 = 8
;; @0090                               v21 = iadd v17, v20
;; @0090                               v22 = iconst.i32 6
;; @0090                               v23 = uextend.i64 v22  ; v22 = 6
;; @0090                               v24 = uextend.i64 v4
;; @0090                               v25 = uextend.i64 v5
;; @0090                               v26 = iconst.i64 1
;; @0090                               v27 = imul v25, v26  ; v26 = 1
;; @0090                               v28 = iadd v24, v27
;; @0090                               v29 = icmp ugt v28, v23
;; @0090                               trapnz v29, user6
;; @0090                               v30 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v31 = uextend.i64 v4
;; @0090                               v32 = iconst.i64 8
;; @0090                               v33 = imul v31, v32  ; v32 = 8
;; @0090                               v34 = iadd v30, v33
;; @0090                               v35 = uextend.i64 v5
;; @0090                               v36 = iconst.i64 8
;; @0090                               v37 = imul v35, v36  ; v36 = 8
;; @0090                               v38 = iconst.i64 8
;; @0090                               v39 = imul v35, v38  ; v38 = 8
;; @0090                               brif v35, block2, block5
;;
;;                                 block2:
;; @0090                               v40 = icmp.i64 ult v21, v34
;; @0090                               v41 = iconst.i64 8
;; @0090                               v42 = imul.i64 v35, v41  ; v41 = 8
;; @0090                               v43 = iconst.i64 8
;; @0090                               v44 = imul.i64 v35, v43  ; v43 = 8
;; @0090                               v45 = iadd.i64 v21, v42
;; @0090                               v46 = iadd.i64 v34, v44
;; @0090                               v47 = ireduce.i32 v35
;; @0090                               v48 = iadd.i32 v4, v47
;; @0090                               brif v40, block3(v21, v34, v4), block4(v45, v46, v48)
;;
;;                                 block3(v49: i64, v50: i64, v51: i32):
;; @0090                               v52 = iconst.i32 6
;; @0090                               v53 = icmp uge v51, v52  ; v52 = 6
;; @0090                               v54 = uextend.i64 v51
;; @0090                               v55 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v56 = iconst.i64 3
;; @0090                               v57 = ishl v54, v56  ; v56 = 3
;; @0090                               v58 = iadd v55, v57
;; @0090                               v59 = iconst.i64 0
;; @0090                               v60 = select_spectre_guard v53, v59, v58  ; v59 = 0
;; @0090                               v61 = load.i64 user6 aligned region5 v60
;; @0090                               v62 = iconst.i64 -2
;; @0090                               v63 = band v61, v62  ; v62 = -2
;; @0090                               brif v61, block7(v63), block6
;;
;;                                 block4(v77: i64, v78: i64, v79: i32):
;; @0090                               v80 = iconst.i64 8
;; @0090                               v81 = isub v77, v80  ; v80 = 8
;; @0090                               v82 = iconst.i64 8
;; @0090                               v83 = isub v78, v82  ; v82 = 8
;; @0090                               v84 = iconst.i32 1
;; @0090                               v85 = isub v79, v84  ; v84 = 1
;; @0090                               v86 = iconst.i32 6
;; @0090                               v87 = icmp uge v85, v86  ; v86 = 6
;; @0090                               v88 = uextend.i64 v85
;; @0090                               v89 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0090                               v90 = iconst.i64 3
;; @0090                               v91 = ishl v88, v90  ; v90 = 3
;; @0090                               v92 = iadd v89, v91
;; @0090                               v93 = iconst.i64 0
;; @0090                               v94 = select_spectre_guard v87, v93, v92  ; v93 = 0
;; @0090                               v95 = load.i64 user6 aligned region5 v94
;; @0090                               v96 = iconst.i64 -2
;; @0090                               v97 = band v95, v96  ; v96 = -2
;; @0090                               brif v95, block9(v97), block8
;;
;;                                 block5:
;; @0094                               jump block1
;;
;;                                 block6 cold:
;; @0090                               v65 = iconst.i32 1
;; @0090                               v66 = uextend.i64 v51
;; @0090                               v67 = call fn0(v0, v65, v66)  ; v65 = 1
;; @0090                               jump block7(v67)
;;
;;                                 block7(v64: i64):
;; @0090                               v68 = iconst.i64 1
;; @0090                               v69 = bor v64, v68  ; v68 = 1
;; @0090                               store notrap aligned region5 v69, v49
;; @0090                               v70 = iconst.i64 8
;; @0090                               v71 = iadd.i64 v49, v70  ; v70 = 8
;; @0090                               v72 = iconst.i64 8
;; @0090                               v73 = iadd.i64 v50, v72  ; v72 = 8
;; @0090                               v74 = iconst.i32 1
;; @0090                               v75 = iadd.i32 v51, v74  ; v74 = 1
;; @0090                               v76 = icmp eq v73, v46
;; @0090                               brif v76, block5, block3(v71, v73, v75)
;;
;;                                 block8 cold:
;; @0090                               v99 = iconst.i32 1
;; @0090                               v100 = uextend.i64 v85
;; @0090                               v101 = call fn0(v0, v99, v100)  ; v99 = 1
;; @0090                               jump block9(v101)
;;
;;                                 block9(v98: i64):
;; @0090                               v102 = iconst.i64 1
;; @0090                               v103 = bor v98, v102  ; v102 = 1
;; @0090                               store notrap aligned region5 v103, v81
;; @0090                               v104 = icmp.i64 eq v83, v34
;; @0090                               brif v104, block5, block4(v81, v83, v85)
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
;; @009f                               v6 = iconst.i32 6
;; @009f                               v7 = uextend.i64 v6  ; v6 = 6
;; @009f                               v8 = uextend.i64 v3
;; @009f                               v9 = uextend.i64 v5
;; @009f                               v10 = iconst.i64 1
;; @009f                               v11 = imul v9, v10  ; v10 = 1
;; @009f                               v12 = iadd v8, v11
;; @009f                               v13 = icmp ugt v12, v7
;; @009f                               trapnz v13, user6
;; @009f                               v14 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @009f                               v15 = uextend.i64 v3
;; @009f                               v16 = iconst.i64 8
;; @009f                               v17 = imul v15, v16  ; v16 = 8
;; @009f                               v18 = iadd v14, v17
;; @009f                               v19 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v20 = load.i64 notrap aligned region4 v19+8
;; @009f                               v21 = ireduce.i32 v20
;; @009f                               v22 = uextend.i64 v21
;; @009f                               v23 = uextend.i64 v4
;; @009f                               v24 = uextend.i64 v5
;; @009f                               v25 = iconst.i64 1
;; @009f                               v26 = imul v24, v25  ; v25 = 1
;; @009f                               v27 = iadd v23, v26
;; @009f                               v28 = icmp ugt v27, v22
;; @009f                               trapnz v28, user6
;; @009f                               v29 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v30 = load.i64 notrap aligned region2 v29
;; @009f                               v31 = uextend.i64 v4
;; @009f                               v32 = iconst.i64 8
;; @009f                               v33 = imul v31, v32  ; v32 = 8
;; @009f                               v34 = iadd v30, v33
;; @009f                               v35 = uextend.i64 v5
;; @009f                               v36 = iconst.i64 8
;; @009f                               v37 = imul v35, v36  ; v36 = 8
;; @009f                               v38 = iconst.i64 8
;; @009f                               v39 = imul v35, v38  ; v38 = 8
;; @009f                               brif v35, block2, block5
;;
;;                                 block2:
;; @009f                               v40 = icmp.i64 ult v18, v34
;; @009f                               v41 = iconst.i64 8
;; @009f                               v42 = imul.i64 v35, v41  ; v41 = 8
;; @009f                               v43 = iconst.i64 8
;; @009f                               v44 = imul.i64 v35, v43  ; v43 = 8
;; @009f                               v45 = iadd.i64 v18, v42
;; @009f                               v46 = iadd.i64 v34, v44
;; @009f                               v47 = ireduce.i32 v35
;; @009f                               v48 = iadd.i32 v4, v47
;; @009f                               brif v40, block3(v18, v34, v4), block4(v45, v46, v48)
;;
;;                                 block3(v49: i64, v50: i64, v51: i32):
;; @009f                               v52 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v53 = load.i64 notrap aligned region4 v52+8
;; @009f                               v54 = ireduce.i32 v53
;; @009f                               v55 = icmp uge v51, v54
;; @009f                               v56 = uextend.i64 v51
;; @009f                               v57 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v58 = load.i64 notrap aligned region2 v57
;; @009f                               v59 = iconst.i64 3
;; @009f                               v60 = ishl v56, v59  ; v59 = 3
;; @009f                               v61 = iadd v58, v60
;; @009f                               v62 = iconst.i64 0
;; @009f                               v63 = select_spectre_guard v55, v62, v61  ; v62 = 0
;; @009f                               v64 = load.i64 user6 aligned region5 v63
;; @009f                               v65 = iconst.i64 -2
;; @009f                               v66 = band v64, v65  ; v65 = -2
;; @009f                               brif v64, block7(v66), block6
;;
;;                                 block4(v80: i64, v81: i64, v82: i32):
;; @009f                               v83 = iconst.i64 8
;; @009f                               v84 = isub v80, v83  ; v83 = 8
;; @009f                               v85 = iconst.i64 8
;; @009f                               v86 = isub v81, v85  ; v85 = 8
;; @009f                               v87 = iconst.i32 1
;; @009f                               v88 = isub v82, v87  ; v87 = 1
;; @009f                               v89 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v90 = load.i64 notrap aligned region4 v89+8
;; @009f                               v91 = ireduce.i32 v90
;; @009f                               v92 = icmp uge v88, v91
;; @009f                               v93 = uextend.i64 v88
;; @009f                               v94 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @009f                               v95 = load.i64 notrap aligned region2 v94
;; @009f                               v96 = iconst.i64 3
;; @009f                               v97 = ishl v93, v96  ; v96 = 3
;; @009f                               v98 = iadd v95, v97
;; @009f                               v99 = iconst.i64 0
;; @009f                               v100 = select_spectre_guard v92, v99, v98  ; v99 = 0
;; @009f                               v101 = load.i64 user6 aligned region5 v100
;; @009f                               v102 = iconst.i64 -2
;; @009f                               v103 = band v101, v102  ; v102 = -2
;; @009f                               brif v101, block9(v103), block8
;;
;;                                 block5:
;; @00a3                               jump block1
;;
;;                                 block6 cold:
;; @009f                               v68 = iconst.i32 0
;; @009f                               v69 = uextend.i64 v51
;; @009f                               v70 = call fn0(v0, v68, v69)  ; v68 = 0
;; @009f                               jump block7(v70)
;;
;;                                 block7(v67: i64):
;; @009f                               v71 = iconst.i64 1
;; @009f                               v72 = bor v67, v71  ; v71 = 1
;; @009f                               store notrap aligned region5 v72, v49
;; @009f                               v73 = iconst.i64 8
;; @009f                               v74 = iadd.i64 v49, v73  ; v73 = 8
;; @009f                               v75 = iconst.i64 8
;; @009f                               v76 = iadd.i64 v50, v75  ; v75 = 8
;; @009f                               v77 = iconst.i32 1
;; @009f                               v78 = iadd.i32 v51, v77  ; v77 = 1
;; @009f                               v79 = icmp eq v76, v46
;; @009f                               brif v79, block5, block3(v74, v76, v78)
;;
;;                                 block8 cold:
;; @009f                               v105 = iconst.i32 0
;; @009f                               v106 = uextend.i64 v88
;; @009f                               v107 = call fn0(v0, v105, v106)  ; v105 = 0
;; @009f                               jump block9(v107)
;;
;;                                 block9(v104: i64):
;; @009f                               v108 = iconst.i64 1
;; @009f                               v109 = bor v104, v108  ; v108 = 1
;; @009f                               store notrap aligned region5 v109, v84
;; @009f                               v110 = icmp.i64 eq v86, v34
;; @009f                               brif v110, block5, block4(v84, v86, v88)
;;
;;                                 block1:
;; @00a3                               return v2
;; }
