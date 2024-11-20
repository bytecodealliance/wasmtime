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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32) -> i32 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v53 = iconst.i64 2
;; @0054                               v8 = ishl v6, v53  ; v53 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user5 aligned table v11
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v12, v54
;;                                     v55 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v55
;;                                     v56 = iconst.i32 0
;; @0054                               v13 = icmp eq v51, v56  ; v56 = 0
;; @0054                               brif v13, block5, block2
;;
;;                                 block2:
;; @0054                               v15 = load.i64 notrap aligned readonly v0+56
;; @0054                               v16 = load.i64 notrap aligned v15
;; @0054                               v17 = load.i64 notrap aligned v15+8
;; @0054                               v18 = icmp eq v16, v17
;; @0054                               brif v18, block3, block4
;;
;;                                 block4:
;; @0054                               v20 = load.i64 notrap aligned readonly v0+40
;; @0054                               v22 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v57
;; @0054                               v23 = uextend.i64 v50
;; @0054                               v24 = iconst.i64 8
;; @0054                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @0054                               v26 = iconst.i64 8
;; @0054                               v27 = uadd_overflow_trap v25, v26, user1  ; v26 = 8
;; @0054                               v28 = icmp ule v27, v22
;; @0054                               trapz v28, user1
;; @0054                               v29 = iadd v20, v25
;; @0054                               v30 = load.i64 notrap aligned v29
;;                                     v58 = iconst.i64 1
;; @0054                               v31 = iadd v30, v58  ; v58 = 1
;; @0054                               v33 = load.i64 notrap aligned readonly v0+40
;; @0054                               v35 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v59
;; @0054                               v36 = uextend.i64 v49
;; @0054                               v37 = iconst.i64 8
;; @0054                               v38 = uadd_overflow_trap v36, v37, user1  ; v37 = 8
;; @0054                               v39 = iconst.i64 8
;; @0054                               v40 = uadd_overflow_trap v38, v39, user1  ; v39 = 8
;; @0054                               v41 = icmp ule v40, v35
;; @0054                               trapz v41, user1
;; @0054                               v42 = iadd v33, v38
;; @0054                               store notrap aligned v31, v42
;;                                     v60 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v60
;; @0054                               store notrap aligned v48, v16
;;                                     v61 = iconst.i64 4
;; @0054                               v43 = iadd.i64 v16, v61  ; v61 = 4
;; @0054                               store notrap aligned v43, v15
;; @0054                               jump block5
;;
;;                                 block3 cold:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v62
;; @0054                               v45 = call fn0(v0, v47), stack_map=[i32 @ ss0+0]
;; @0054                               jump block5
;;
;;                                 block5:
;;                                     v63 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v63
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v46
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32) -> i32 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v53 = iconst.i64 2
;; @005b                               v8 = ishl v6, v53  ; v53 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user5 aligned table v11
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v12, v54
;;                                     v55 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v55
;;                                     v56 = iconst.i32 0
;; @005b                               v13 = icmp eq v51, v56  ; v56 = 0
;; @005b                               brif v13, block5, block2
;;
;;                                 block2:
;; @005b                               v15 = load.i64 notrap aligned readonly v0+56
;; @005b                               v16 = load.i64 notrap aligned v15
;; @005b                               v17 = load.i64 notrap aligned v15+8
;; @005b                               v18 = icmp eq v16, v17
;; @005b                               brif v18, block3, block4
;;
;;                                 block4:
;; @005b                               v20 = load.i64 notrap aligned readonly v0+40
;; @005b                               v22 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v57
;; @005b                               v23 = uextend.i64 v50
;; @005b                               v24 = iconst.i64 8
;; @005b                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @005b                               v26 = iconst.i64 8
;; @005b                               v27 = uadd_overflow_trap v25, v26, user1  ; v26 = 8
;; @005b                               v28 = icmp ule v27, v22
;; @005b                               trapz v28, user1
;; @005b                               v29 = iadd v20, v25
;; @005b                               v30 = load.i64 notrap aligned v29
;;                                     v58 = iconst.i64 1
;; @005b                               v31 = iadd v30, v58  ; v58 = 1
;; @005b                               v33 = load.i64 notrap aligned readonly v0+40
;; @005b                               v35 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v59
;; @005b                               v36 = uextend.i64 v49
;; @005b                               v37 = iconst.i64 8
;; @005b                               v38 = uadd_overflow_trap v36, v37, user1  ; v37 = 8
;; @005b                               v39 = iconst.i64 8
;; @005b                               v40 = uadd_overflow_trap v38, v39, user1  ; v39 = 8
;; @005b                               v41 = icmp ule v40, v35
;; @005b                               trapz v41, user1
;; @005b                               v42 = iadd v33, v38
;; @005b                               store notrap aligned v31, v42
;;                                     v60 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v60
;; @005b                               store notrap aligned v48, v16
;;                                     v61 = iconst.i64 4
;; @005b                               v43 = iadd.i64 v16, v61  ; v61 = 4
;; @005b                               store notrap aligned v43, v15
;; @005b                               jump block5
;;
;;                                 block3 cold:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v62
;; @005b                               v45 = call fn0(v0, v47), stack_map=[i32 @ ss0+0]
;; @005b                               jump block5
;;
;;                                 block5:
;;                                     v63 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v63
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v46
;; }
