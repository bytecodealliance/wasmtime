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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v57 = iconst.i64 2
;; @0054                               v8 = ishl v6, v57  ; v57 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user5 aligned table v11
;;                                     v58 = stack_addr.i64 ss0
;;                                     store notrap v12, v58
;;                                     v59 = stack_addr.i64 ss0
;;                                     v55 = load.i32 notrap v59
;;                                     v60 = iconst.i32 1
;; @0054                               v13 = band v55, v60  ; v60 = 1
;;                                     v61 = stack_addr.i64 ss0
;;                                     v54 = load.i32 notrap v61
;;                                     v62 = iconst.i32 0
;; @0054                               v14 = icmp eq v54, v62  ; v62 = 0
;; @0054                               v15 = uextend.i32 v14
;; @0054                               v16 = bor v13, v15
;; @0054                               brif v16, block5, block2
;;
;;                                 block2:
;; @0054                               v18 = load.i64 notrap aligned readonly v0+56
;; @0054                               v19 = load.i64 notrap aligned v18
;; @0054                               v20 = load.i64 notrap aligned v18+8
;; @0054                               v21 = icmp eq v19, v20
;; @0054                               brif v21, block3, block4
;;
;;                                 block4:
;; @0054                               v23 = load.i64 notrap aligned readonly can_move v0+40
;; @0054                               v25 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v63 = stack_addr.i64 ss0
;;                                     v53 = load.i32 notrap v63
;; @0054                               v26 = uextend.i64 v53
;; @0054                               v27 = iconst.i64 8
;; @0054                               v28 = uadd_overflow_trap v26, v27, user1  ; v27 = 8
;; @0054                               v29 = iconst.i64 8
;; @0054                               v30 = uadd_overflow_trap v28, v29, user1  ; v29 = 8
;; @0054                               v31 = icmp ule v30, v25
;; @0054                               trapz v31, user1
;; @0054                               v32 = iadd v23, v28
;; @0054                               v33 = load.i64 notrap aligned v32
;;                                     v64 = iconst.i64 1
;; @0054                               v34 = iadd v33, v64  ; v64 = 1
;; @0054                               v36 = load.i64 notrap aligned readonly can_move v0+40
;; @0054                               v38 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v65 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v65
;; @0054                               v39 = uextend.i64 v52
;; @0054                               v40 = iconst.i64 8
;; @0054                               v41 = uadd_overflow_trap v39, v40, user1  ; v40 = 8
;; @0054                               v42 = iconst.i64 8
;; @0054                               v43 = uadd_overflow_trap v41, v42, user1  ; v42 = 8
;; @0054                               v44 = icmp ule v43, v38
;; @0054                               trapz v44, user1
;; @0054                               v45 = iadd v36, v41
;; @0054                               store notrap aligned v34, v45
;;                                     v66 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v66
;; @0054                               store notrap aligned v51, v19
;;                                     v67 = iconst.i64 4
;; @0054                               v46 = iadd.i64 v19, v67  ; v67 = 4
;; @0054                               store notrap aligned v46, v18
;; @0054                               jump block5
;;
;;                                 block3 cold:
;;                                     v68 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v68
;; @0054                               v48 = call fn0(v0, v50), stack_map=[i32 @ ss0+0]
;; @0054                               jump block5
;;
;;                                 block5:
;;                                     v69 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v69
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v49
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v57 = iconst.i64 2
;; @005b                               v8 = ishl v6, v57  ; v57 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user5 aligned table v11
;;                                     v58 = stack_addr.i64 ss0
;;                                     store notrap v12, v58
;;                                     v59 = stack_addr.i64 ss0
;;                                     v55 = load.i32 notrap v59
;;                                     v60 = iconst.i32 1
;; @005b                               v13 = band v55, v60  ; v60 = 1
;;                                     v61 = stack_addr.i64 ss0
;;                                     v54 = load.i32 notrap v61
;;                                     v62 = iconst.i32 0
;; @005b                               v14 = icmp eq v54, v62  ; v62 = 0
;; @005b                               v15 = uextend.i32 v14
;; @005b                               v16 = bor v13, v15
;; @005b                               brif v16, block5, block2
;;
;;                                 block2:
;; @005b                               v18 = load.i64 notrap aligned readonly v0+56
;; @005b                               v19 = load.i64 notrap aligned v18
;; @005b                               v20 = load.i64 notrap aligned v18+8
;; @005b                               v21 = icmp eq v19, v20
;; @005b                               brif v21, block3, block4
;;
;;                                 block4:
;; @005b                               v23 = load.i64 notrap aligned readonly can_move v0+40
;; @005b                               v25 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v63 = stack_addr.i64 ss0
;;                                     v53 = load.i32 notrap v63
;; @005b                               v26 = uextend.i64 v53
;; @005b                               v27 = iconst.i64 8
;; @005b                               v28 = uadd_overflow_trap v26, v27, user1  ; v27 = 8
;; @005b                               v29 = iconst.i64 8
;; @005b                               v30 = uadd_overflow_trap v28, v29, user1  ; v29 = 8
;; @005b                               v31 = icmp ule v30, v25
;; @005b                               trapz v31, user1
;; @005b                               v32 = iadd v23, v28
;; @005b                               v33 = load.i64 notrap aligned v32
;;                                     v64 = iconst.i64 1
;; @005b                               v34 = iadd v33, v64  ; v64 = 1
;; @005b                               v36 = load.i64 notrap aligned readonly can_move v0+40
;; @005b                               v38 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v65 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v65
;; @005b                               v39 = uextend.i64 v52
;; @005b                               v40 = iconst.i64 8
;; @005b                               v41 = uadd_overflow_trap v39, v40, user1  ; v40 = 8
;; @005b                               v42 = iconst.i64 8
;; @005b                               v43 = uadd_overflow_trap v41, v42, user1  ; v42 = 8
;; @005b                               v44 = icmp ule v43, v38
;; @005b                               trapz v44, user1
;; @005b                               v45 = iadd v36, v41
;; @005b                               store notrap aligned v34, v45
;;                                     v66 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v66
;; @005b                               store notrap aligned v51, v19
;;                                     v67 = iconst.i64 4
;; @005b                               v46 = iadd.i64 v19, v67  ; v67 = 4
;; @005b                               store notrap aligned v46, v18
;; @005b                               jump block5
;;
;;                                 block3 cold:
;;                                     v68 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v68
;; @005b                               v48 = call fn0(v0, v50), stack_map=[i32 @ ss0+0]
;; @005b                               jump block5
;;
;;                                 block5:
;;                                     v69 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v69
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v49
;; }
