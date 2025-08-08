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

;; function u0:0(i64 vmctx, i64) -> i32, i32, i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v113 = iconst.i64 80
;; @008f                               v7 = iadd v0, v113  ; v113 = 80
;; @008f                               v8 = load.i32 notrap aligned readonly can_move v7
;;                                     v112 = iconst.i32 1
;; @008f                               v9 = band v8, v112  ; v112 = 1
;;                                     v111 = iconst.i32 0
;; @008f                               v10 = icmp eq v8, v111  ; v111 = 0
;; @008f                               v11 = uextend.i32 v10
;; @008f                               v12 = bor v9, v11
;; @008f                               brif v12, block4, block2
;;
;;                                 block2:
;; @008f                               v13 = uextend.i64 v8
;; @008f                               v109 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v14 = load.i64 notrap aligned readonly can_move v109+24
;; @008f                               v15 = iadd v14, v13
;; @008f                               v16 = load.i32 notrap aligned v15
;; @008f                               v17 = iconst.i32 2
;; @008f                               v18 = band v16, v17  ; v17 = 2
;; @008f                               brif v18, block4, block3
;;
;;                                 block3:
;; @008f                               v20 = load.i64 notrap aligned readonly v0+32
;; @008f                               v21 = load.i32 notrap aligned v20
;; @008f                               v22 = uextend.i64 v8
;; @008f                               v107 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v23 = load.i64 notrap aligned readonly can_move v107+24
;; @008f                               v24 = iadd v23, v22
;; @008f                               v25 = iconst.i64 16
;; @008f                               v26 = iadd v24, v25  ; v25 = 16
;; @008f                               store notrap aligned v21, v26
;; @008f                               v27 = iconst.i32 2
;; @008f                               v28 = bor.i32 v16, v27  ; v27 = 2
;; @008f                               v29 = uextend.i64 v8
;; @008f                               v105 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v30 = load.i64 notrap aligned readonly can_move v105+24
;; @008f                               v31 = iadd v30, v29
;; @008f                               store notrap aligned v28, v31
;; @008f                               v32 = uextend.i64 v8
;; @008f                               v103 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v33 = load.i64 notrap aligned readonly can_move v103+24
;; @008f                               v34 = iadd v33, v32
;; @008f                               v35 = iconst.i64 8
;; @008f                               v36 = iadd v34, v35  ; v35 = 8
;; @008f                               v37 = load.i64 notrap aligned v36
;;                                     v102 = iconst.i64 1
;; @008f                               v38 = iadd v37, v102  ; v102 = 1
;; @008f                               v39 = uextend.i64 v8
;; @008f                               v100 = load.i64 notrap aligned readonly can_move v0+8
;; @008f                               v40 = load.i64 notrap aligned readonly can_move v100+24
;; @008f                               v41 = iadd v40, v39
;; @008f                               v42 = iconst.i64 8
;; @008f                               v43 = iadd v41, v42  ; v42 = 8
;; @008f                               store notrap aligned v38, v43
;; @008f                               store.i32 notrap aligned v8, v20
;; @008f                               jump block4
;;
;;                                 block4:
;;                                     v99 = iconst.i64 96
;; @0091                               v45 = iadd.i64 v0, v99  ; v99 = 96
;; @0091                               v46 = load.i32 notrap aligned readonly can_move v45
;;                                     v98 = iconst.i32 1
;; @0091                               v47 = band v46, v98  ; v98 = 1
;;                                     v97 = iconst.i32 0
;; @0091                               v48 = icmp eq v46, v97  ; v97 = 0
;; @0091                               v49 = uextend.i32 v48
;; @0091                               v50 = bor v47, v49
;; @0091                               brif v50, block7, block5
;;
;;                                 block5:
;; @0091                               v51 = uextend.i64 v46
;; @0091                               v95 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v52 = load.i64 notrap aligned readonly can_move v95+24
;; @0091                               v53 = iadd v52, v51
;; @0091                               v54 = load.i32 notrap aligned v53
;; @0091                               v55 = iconst.i32 2
;; @0091                               v56 = band v54, v55  ; v55 = 2
;; @0091                               brif v56, block7, block6
;;
;;                                 block6:
;; @0091                               v58 = load.i64 notrap aligned readonly v0+32
;; @0091                               v59 = load.i32 notrap aligned v58
;; @0091                               v60 = uextend.i64 v46
;; @0091                               v93 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v61 = load.i64 notrap aligned readonly can_move v93+24
;; @0091                               v62 = iadd v61, v60
;; @0091                               v63 = iconst.i64 16
;; @0091                               v64 = iadd v62, v63  ; v63 = 16
;; @0091                               store notrap aligned v59, v64
;; @0091                               v65 = iconst.i32 2
;; @0091                               v66 = bor.i32 v54, v65  ; v65 = 2
;; @0091                               v67 = uextend.i64 v46
;; @0091                               v91 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v68 = load.i64 notrap aligned readonly can_move v91+24
;; @0091                               v69 = iadd v68, v67
;; @0091                               store notrap aligned v66, v69
;; @0091                               v70 = uextend.i64 v46
;; @0091                               v89 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v71 = load.i64 notrap aligned readonly can_move v89+24
;; @0091                               v72 = iadd v71, v70
;; @0091                               v73 = iconst.i64 8
;; @0091                               v74 = iadd v72, v73  ; v73 = 8
;; @0091                               v75 = load.i64 notrap aligned v74
;;                                     v88 = iconst.i64 1
;; @0091                               v76 = iadd v75, v88  ; v88 = 1
;; @0091                               v77 = uextend.i64 v46
;; @0091                               v86 = load.i64 notrap aligned readonly can_move v0+8
;; @0091                               v78 = load.i64 notrap aligned readonly can_move v86+24
;; @0091                               v79 = iadd v78, v77
;; @0091                               v80 = iconst.i64 8
;; @0091                               v81 = iadd v79, v80  ; v80 = 8
;; @0091                               store notrap aligned v76, v81
;; @0091                               store.i32 notrap aligned v46, v58
;; @0091                               jump block7
;;
;;                                 block7:
;; @0093                               v83 = load.i64 notrap aligned table v0+112
;; @0095                               v85 = load.i64 notrap aligned table v0+128
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v8, v46, v83, v85
;; }
