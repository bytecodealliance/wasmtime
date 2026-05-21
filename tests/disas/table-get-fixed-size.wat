;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimized but with `opt-level=0` to legalize away
;; macro instructions.

(module
  (table (export "table") 7 7 externref)
  (func (export "table.get.const") (result externref)
    i32.const 0
    table.get 0)
  (func (export "table.get.var") (param i32) (result externref)
    local.get 0
    table.get 0))

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+32
;;     gv7 = load.i64 notrap aligned gv5+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:46 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v99 = iconst.i64 2
;; @0054                               v8 = ishl v6, v99  ; v99 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user6 aligned table v11
;;                                     v98 = stack_addr.i64 ss0
;;                                     store notrap v12, v98
;;                                     v97 = stack_addr.i64 ss0
;;                                     v74 = load.i32 notrap v97
;;                                     v96 = iconst.i32 1
;; @0054                               v13 = band v74, v96  ; v96 = 1
;;                                     v95 = stack_addr.i64 ss0
;;                                     v73 = load.i32 notrap v95
;;                                     v94 = iconst.i32 0
;; @0054                               v14 = icmp eq v73, v94  ; v94 = 0
;; @0054                               v15 = uextend.i32 v14
;; @0054                               v16 = bor v13, v15
;; @0054                               brif v16, block4, block2
;;
;;                                 block2:
;;                                     v93 = stack_addr.i64 ss0
;;                                     v72 = load.i32 notrap v93
;; @0054                               v17 = uextend.i64 v72
;; @0054                               v91 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v18 = load.i64 notrap aligned readonly can_move v91+32
;; @0054                               v19 = iadd v18, v17
;; @0054                               v20 = load.i32 user2 v19
;; @0054                               v21 = iconst.i32 2
;; @0054                               v22 = band v20, v21  ; v21 = 2
;; @0054                               brif v22, block4, block3
;;
;;                                 block3:
;; @0054                               v24 = load.i64 notrap aligned readonly can_move v0+32
;; @0054                               v25 = load.i32 user2 v24
;;                                     v90 = stack_addr.i64 ss0
;;                                     v71 = load.i32 notrap v90
;; @0054                               v26 = uextend.i64 v71
;; @0054                               v88 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v27 = load.i64 notrap aligned readonly can_move v88+32
;; @0054                               v28 = iadd v27, v26
;; @0054                               v29 = iconst.i64 16
;; @0054                               v30 = iadd v28, v29  ; v29 = 16
;; @0054                               store user2 v25, v30
;; @0054                               v31 = iconst.i32 2
;; @0054                               v32 = bor.i32 v20, v31  ; v31 = 2
;;                                     v87 = stack_addr.i64 ss0
;;                                     v70 = load.i32 notrap v87
;; @0054                               v33 = uextend.i64 v70
;; @0054                               v85 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v34 = load.i64 notrap aligned readonly can_move v85+32
;; @0054                               v35 = iadd v34, v33
;; @0054                               store user2 v32, v35
;;                                     v84 = stack_addr.i64 ss0
;;                                     v69 = load.i32 notrap v84
;; @0054                               v36 = uextend.i64 v69
;; @0054                               v82 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v37 = load.i64 notrap aligned readonly can_move v82+32
;; @0054                               v38 = iadd v37, v36
;; @0054                               v39 = iconst.i64 8
;; @0054                               v40 = iadd v38, v39  ; v39 = 8
;; @0054                               v41 = load.i64 user2 v40
;;                                     v81 = iconst.i64 1
;; @0054                               v42 = iadd v41, v81  ; v81 = 1
;;                                     v80 = stack_addr.i64 ss0
;;                                     v68 = load.i32 notrap v80
;; @0054                               v43 = uextend.i64 v68
;; @0054                               v78 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v44 = load.i64 notrap aligned readonly can_move v78+32
;; @0054                               v45 = iadd v44, v43
;; @0054                               v46 = iconst.i64 8
;; @0054                               v47 = iadd v45, v46  ; v46 = 8
;; @0054                               store user2 v42, v47
;;                                     v77 = stack_addr.i64 ss0
;;                                     v67 = load.i32 notrap v77
;; @0054                               store user2 v67, v24
;; @0054                               v49 = load.i64 notrap aligned readonly can_move v0+32
;; @0054                               v50 = load.i32 notrap aligned v49+4
;;                                     v76 = iconst.i32 1
;; @0054                               v51 = iadd v50, v76  ; v76 = 1
;; @0054                               v53 = load.i64 notrap aligned readonly can_move v0+32
;; @0054                               store notrap aligned v51, v53+4
;; @0054                               v55 = load.i64 notrap aligned readonly can_move v0+32
;; @0054                               v56 = load.i32 notrap aligned v55+4
;; @0054                               v58 = load.i64 notrap aligned readonly can_move v0+32
;; @0054                               v59 = load.i32 notrap aligned v58+8
;; @0054                               v60 = iadd v59, v59
;; @0054                               v61 = iconst.i32 1024
;; @0054                               v62 = umax v60, v61  ; v61 = 1024
;; @0054                               v63 = icmp uge v56, v62
;; @0054                               brif v63, block5, block6
;;
;;                                 block5 cold:
;; @0054                               v65 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @0054                               jump block6
;;
;;                                 block6:
;; @0054                               jump block4
;;
;;                                 block4:
;;                                     v75 = stack_addr.i64 ss0
;;                                     v66 = load.i32 notrap v75
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v66
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+32
;;     gv7 = load.i64 notrap aligned gv5+40
;;     sig0 = (i64 vmctx) -> i8 tail
;;     fn0 = colocated u805306368:46 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v99 = iconst.i64 2
;; @005b                               v8 = ishl v6, v99  ; v99 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user6 aligned table v11
;;                                     v98 = stack_addr.i64 ss0
;;                                     store notrap v12, v98
;;                                     v97 = stack_addr.i64 ss0
;;                                     v74 = load.i32 notrap v97
;;                                     v96 = iconst.i32 1
;; @005b                               v13 = band v74, v96  ; v96 = 1
;;                                     v95 = stack_addr.i64 ss0
;;                                     v73 = load.i32 notrap v95
;;                                     v94 = iconst.i32 0
;; @005b                               v14 = icmp eq v73, v94  ; v94 = 0
;; @005b                               v15 = uextend.i32 v14
;; @005b                               v16 = bor v13, v15
;; @005b                               brif v16, block4, block2
;;
;;                                 block2:
;;                                     v93 = stack_addr.i64 ss0
;;                                     v72 = load.i32 notrap v93
;; @005b                               v17 = uextend.i64 v72
;; @005b                               v91 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v18 = load.i64 notrap aligned readonly can_move v91+32
;; @005b                               v19 = iadd v18, v17
;; @005b                               v20 = load.i32 user2 v19
;; @005b                               v21 = iconst.i32 2
;; @005b                               v22 = band v20, v21  ; v21 = 2
;; @005b                               brif v22, block4, block3
;;
;;                                 block3:
;; @005b                               v24 = load.i64 notrap aligned readonly can_move v0+32
;; @005b                               v25 = load.i32 user2 v24
;;                                     v90 = stack_addr.i64 ss0
;;                                     v71 = load.i32 notrap v90
;; @005b                               v26 = uextend.i64 v71
;; @005b                               v88 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v27 = load.i64 notrap aligned readonly can_move v88+32
;; @005b                               v28 = iadd v27, v26
;; @005b                               v29 = iconst.i64 16
;; @005b                               v30 = iadd v28, v29  ; v29 = 16
;; @005b                               store user2 v25, v30
;; @005b                               v31 = iconst.i32 2
;; @005b                               v32 = bor.i32 v20, v31  ; v31 = 2
;;                                     v87 = stack_addr.i64 ss0
;;                                     v70 = load.i32 notrap v87
;; @005b                               v33 = uextend.i64 v70
;; @005b                               v85 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v34 = load.i64 notrap aligned readonly can_move v85+32
;; @005b                               v35 = iadd v34, v33
;; @005b                               store user2 v32, v35
;;                                     v84 = stack_addr.i64 ss0
;;                                     v69 = load.i32 notrap v84
;; @005b                               v36 = uextend.i64 v69
;; @005b                               v82 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v37 = load.i64 notrap aligned readonly can_move v82+32
;; @005b                               v38 = iadd v37, v36
;; @005b                               v39 = iconst.i64 8
;; @005b                               v40 = iadd v38, v39  ; v39 = 8
;; @005b                               v41 = load.i64 user2 v40
;;                                     v81 = iconst.i64 1
;; @005b                               v42 = iadd v41, v81  ; v81 = 1
;;                                     v80 = stack_addr.i64 ss0
;;                                     v68 = load.i32 notrap v80
;; @005b                               v43 = uextend.i64 v68
;; @005b                               v78 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v44 = load.i64 notrap aligned readonly can_move v78+32
;; @005b                               v45 = iadd v44, v43
;; @005b                               v46 = iconst.i64 8
;; @005b                               v47 = iadd v45, v46  ; v46 = 8
;; @005b                               store user2 v42, v47
;;                                     v77 = stack_addr.i64 ss0
;;                                     v67 = load.i32 notrap v77
;; @005b                               store user2 v67, v24
;; @005b                               v49 = load.i64 notrap aligned readonly can_move v0+32
;; @005b                               v50 = load.i32 notrap aligned v49+4
;;                                     v76 = iconst.i32 1
;; @005b                               v51 = iadd v50, v76  ; v76 = 1
;; @005b                               v53 = load.i64 notrap aligned readonly can_move v0+32
;; @005b                               store notrap aligned v51, v53+4
;; @005b                               v55 = load.i64 notrap aligned readonly can_move v0+32
;; @005b                               v56 = load.i32 notrap aligned v55+4
;; @005b                               v58 = load.i64 notrap aligned readonly can_move v0+32
;; @005b                               v59 = load.i32 notrap aligned v58+8
;; @005b                               v60 = iadd v59, v59
;; @005b                               v61 = iconst.i32 1024
;; @005b                               v62 = umax v60, v61  ; v61 = 1024
;; @005b                               v63 = icmp uge v56, v62
;; @005b                               brif v63, block5, block6
;;
;;                                 block5 cold:
;; @005b                               v65 = call fn0(v0), stack_map=[i32 @ ss0+0]
;; @005b                               jump block6
;;
;;                                 block6:
;; @005b                               jump block4
;;
;;                                 block4:
;;                                     v75 = stack_addr.i64 ss0
;;                                     v66 = load.i32 notrap v75
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v66
;; }
