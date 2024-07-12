;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with `opt-level=0` to legalize away table_addr instructions.

(module
  (table (export "table") 1 externref)
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
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i32 notrap aligned v0+96
;; @0053                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0053                               v6 = uextend.i64 v3  ; v3 = 0
;; @0053                               v7 = load.i64 notrap aligned v0+88
;;                                     v52 = iconst.i64 2
;; @0053                               v8 = ishl v6, v52  ; v52 = 2
;; @0053                               v9 = iadd v7, v8
;; @0053                               v10 = iconst.i64 0
;; @0053                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0053                               v12 = load.i32 table_oob aligned table v11
;;                                     v53 = stack_addr.i64 ss0
;;                                     store notrap v12, v53
;;                                     v54 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v54
;;                                     v55 = iconst.i32 0
;; @0053                               v13 = icmp eq v49, v55  ; v55 = 0
;; @0053                               brif v13, block5, block2
;;
;;                                 block2:
;; @0053                               v15 = load.i64 notrap aligned v0+56
;; @0053                               v16 = load.i64 notrap aligned v15
;; @0053                               v17 = load.i64 notrap aligned v15+8
;; @0053                               v18 = icmp eq v16, v17
;; @0053                               brif v18, block3, block4
;;
;;                                 block4:
;; @0053                               v20 = load.i64 notrap aligned readonly v0+40
;; @0053                               v21 = load.i64 notrap aligned readonly v0+48
;;                                     v56 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v56
;; @0053                               v22 = uextend.i64 v48
;; @0053                               v23 = iconst.i64 8
;; @0053                               v24 = uadd_overflow_trap v22, v23, user65535  ; v23 = 8
;; @0053                               v25 = iconst.i64 8
;; @0053                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @0053                               v27 = icmp ult v26, v21
;; @0053                               brif v27, block7, block6
;;
;;                                 block6 cold:
;; @0053                               trap user65535
;;
;;                                 block7:
;; @0053                               v28 = iadd.i64 v20, v24
;; @0053                               v29 = load.i64 notrap aligned v28
;;                                     v57 = iconst.i64 1
;; @0053                               v30 = iadd v29, v57  ; v57 = 1
;; @0053                               v32 = load.i64 notrap aligned readonly v0+40
;; @0053                               v33 = load.i64 notrap aligned readonly v0+48
;;                                     v58 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v58
;; @0053                               v34 = uextend.i64 v47
;; @0053                               v35 = iconst.i64 8
;; @0053                               v36 = uadd_overflow_trap v34, v35, user65535  ; v35 = 8
;; @0053                               v37 = iconst.i64 8
;; @0053                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @0053                               v39 = icmp ult v38, v33
;; @0053                               brif v39, block9, block8
;;
;;                                 block8 cold:
;; @0053                               trap user65535
;;
;;                                 block9:
;; @0053                               v40 = iadd.i64 v32, v36
;; @0053                               store.i64 notrap aligned v30, v40
;;                                     v59 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v59
;; @0053                               store notrap aligned v46, v16
;;                                     v60 = iconst.i64 4
;; @0053                               v41 = iadd.i64 v16, v60  ; v60 = 4
;; @0053                               store notrap aligned v41, v15
;; @0053                               jump block5
;;
;;                                 block3 cold:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v61
;; @0053                               v43 = call fn0(v0, v45), stack_map=[i32 @ ss0+0]
;; @0053                               jump block5
;;
;;                                 block5:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v62
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v44
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i32 notrap aligned v0+96
;; @005a                               v5 = icmp uge v2, v4
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v7 = load.i64 notrap aligned v0+88
;;                                     v52 = iconst.i64 2
;; @005a                               v8 = ishl v6, v52  ; v52 = 2
;; @005a                               v9 = iadd v7, v8
;; @005a                               v10 = iconst.i64 0
;; @005a                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005a                               v12 = load.i32 table_oob aligned table v11
;;                                     v53 = stack_addr.i64 ss0
;;                                     store notrap v12, v53
;;                                     v54 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v54
;;                                     v55 = iconst.i32 0
;; @005a                               v13 = icmp eq v49, v55  ; v55 = 0
;; @005a                               brif v13, block5, block2
;;
;;                                 block2:
;; @005a                               v15 = load.i64 notrap aligned v0+56
;; @005a                               v16 = load.i64 notrap aligned v15
;; @005a                               v17 = load.i64 notrap aligned v15+8
;; @005a                               v18 = icmp eq v16, v17
;; @005a                               brif v18, block3, block4
;;
;;                                 block4:
;; @005a                               v20 = load.i64 notrap aligned readonly v0+40
;; @005a                               v21 = load.i64 notrap aligned readonly v0+48
;;                                     v56 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v56
;; @005a                               v22 = uextend.i64 v48
;; @005a                               v23 = iconst.i64 8
;; @005a                               v24 = uadd_overflow_trap v22, v23, user65535  ; v23 = 8
;; @005a                               v25 = iconst.i64 8
;; @005a                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @005a                               v27 = icmp ult v26, v21
;; @005a                               brif v27, block7, block6
;;
;;                                 block6 cold:
;; @005a                               trap user65535
;;
;;                                 block7:
;; @005a                               v28 = iadd.i64 v20, v24
;; @005a                               v29 = load.i64 notrap aligned v28
;;                                     v57 = iconst.i64 1
;; @005a                               v30 = iadd v29, v57  ; v57 = 1
;; @005a                               v32 = load.i64 notrap aligned readonly v0+40
;; @005a                               v33 = load.i64 notrap aligned readonly v0+48
;;                                     v58 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v58
;; @005a                               v34 = uextend.i64 v47
;; @005a                               v35 = iconst.i64 8
;; @005a                               v36 = uadd_overflow_trap v34, v35, user65535  ; v35 = 8
;; @005a                               v37 = iconst.i64 8
;; @005a                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @005a                               v39 = icmp ult v38, v33
;; @005a                               brif v39, block9, block8
;;
;;                                 block8 cold:
;; @005a                               trap user65535
;;
;;                                 block9:
;; @005a                               v40 = iadd.i64 v32, v36
;; @005a                               store.i64 notrap aligned v30, v40
;;                                     v59 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v59
;; @005a                               store notrap aligned v46, v16
;;                                     v60 = iconst.i64 4
;; @005a                               v41 = iadd.i64 v16, v60  ; v60 = 4
;; @005a                               store notrap aligned v41, v15
;; @005a                               jump block5
;;
;;                                 block3 cold:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v61
;; @005a                               v43 = call fn0(v0, v45), stack_map=[i32 @ ss0+0]
;; @005a                               jump block5
;;
;;                                 block5:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v62
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v44
;; }
