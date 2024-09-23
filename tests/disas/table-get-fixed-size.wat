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
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v51 = iconst.i64 2
;; @0054                               v8 = ishl v6, v51  ; v51 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 table_oob aligned table v11
;;                                     v52 = stack_addr.i64 ss0
;;                                     store notrap v12, v52
;;                                     v53 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v53
;;                                     v54 = iconst.i32 0
;; @0054                               v13 = icmp eq v49, v54  ; v54 = 0
;; @0054                               brif v13, block5, block2
;;
;;                                 block2:
;; @0054                               v15 = load.i64 notrap aligned v0+56
;; @0054                               v16 = load.i64 notrap aligned v15
;; @0054                               v17 = load.i64 notrap aligned v15+8
;; @0054                               v18 = icmp eq v16, v17
;; @0054                               brif v18, block3, block4
;;
;;                                 block4:
;; @0054                               v20 = load.i64 notrap aligned readonly v0+40
;; @0054                               v21 = load.i64 notrap aligned readonly v0+48
;;                                     v55 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v55
;; @0054                               v22 = uextend.i64 v48
;; @0054                               v23 = iconst.i64 8
;; @0054                               v24 = uadd_overflow_trap v22, v23, user65535  ; v23 = 8
;; @0054                               v25 = iconst.i64 8
;; @0054                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @0054                               v27 = icmp ult v26, v21
;; @0054                               trapz v27, user65535
;; @0054                               v28 = iadd v20, v24
;; @0054                               v29 = load.i64 notrap aligned v28
;;                                     v56 = iconst.i64 1
;; @0054                               v30 = iadd v29, v56  ; v56 = 1
;; @0054                               v32 = load.i64 notrap aligned readonly v0+40
;; @0054                               v33 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v57
;; @0054                               v34 = uextend.i64 v47
;; @0054                               v35 = iconst.i64 8
;; @0054                               v36 = uadd_overflow_trap v34, v35, user65535  ; v35 = 8
;; @0054                               v37 = iconst.i64 8
;; @0054                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @0054                               v39 = icmp ult v38, v33
;; @0054                               trapz v39, user65535
;; @0054                               v40 = iadd v32, v36
;; @0054                               store notrap aligned v30, v40
;;                                     v58 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v58
;; @0054                               store notrap aligned v46, v16
;;                                     v59 = iconst.i64 4
;; @0054                               v41 = iadd.i64 v16, v59  ; v59 = 4
;; @0054                               store notrap aligned v41, v15
;; @0054                               jump block5
;;
;;                                 block3 cold:
;;                                     v60 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v60
;; @0054                               v43 = call fn0(v0, v45), stack_map=[i32 @ ss0+0]
;; @0054                               jump block5
;;
;;                                 block5:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v61
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v44
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v51 = iconst.i64 2
;; @005b                               v8 = ishl v6, v51  ; v51 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 table_oob aligned table v11
;;                                     v52 = stack_addr.i64 ss0
;;                                     store notrap v12, v52
;;                                     v53 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v53
;;                                     v54 = iconst.i32 0
;; @005b                               v13 = icmp eq v49, v54  ; v54 = 0
;; @005b                               brif v13, block5, block2
;;
;;                                 block2:
;; @005b                               v15 = load.i64 notrap aligned v0+56
;; @005b                               v16 = load.i64 notrap aligned v15
;; @005b                               v17 = load.i64 notrap aligned v15+8
;; @005b                               v18 = icmp eq v16, v17
;; @005b                               brif v18, block3, block4
;;
;;                                 block4:
;; @005b                               v20 = load.i64 notrap aligned readonly v0+40
;; @005b                               v21 = load.i64 notrap aligned readonly v0+48
;;                                     v55 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v55
;; @005b                               v22 = uextend.i64 v48
;; @005b                               v23 = iconst.i64 8
;; @005b                               v24 = uadd_overflow_trap v22, v23, user65535  ; v23 = 8
;; @005b                               v25 = iconst.i64 8
;; @005b                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @005b                               v27 = icmp ult v26, v21
;; @005b                               trapz v27, user65535
;; @005b                               v28 = iadd v20, v24
;; @005b                               v29 = load.i64 notrap aligned v28
;;                                     v56 = iconst.i64 1
;; @005b                               v30 = iadd v29, v56  ; v56 = 1
;; @005b                               v32 = load.i64 notrap aligned readonly v0+40
;; @005b                               v33 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v57
;; @005b                               v34 = uextend.i64 v47
;; @005b                               v35 = iconst.i64 8
;; @005b                               v36 = uadd_overflow_trap v34, v35, user65535  ; v35 = 8
;; @005b                               v37 = iconst.i64 8
;; @005b                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @005b                               v39 = icmp ult v38, v33
;; @005b                               trapz v39, user65535
;; @005b                               v40 = iadd v32, v36
;; @005b                               store notrap aligned v30, v40
;;                                     v58 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v58
;; @005b                               store notrap aligned v46, v16
;;                                     v59 = iconst.i64 4
;; @005b                               v41 = iadd.i64 v16, v59  ; v59 = 4
;; @005b                               store notrap aligned v41, v15
;; @005b                               jump block5
;;
;;                                 block3 cold:
;;                                     v60 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v60
;; @005b                               v43 = call fn0(v0, v45), stack_map=[i32 @ ss0+0]
;; @005b                               jump block5
;;
;;                                 block5:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v61
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v44
;; }
