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
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+96
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+88
;;                                     v53 = iconst.i64 2
;; @0053                               v9 = ishl v7, v53  ; v53 = 2
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 table_oob aligned table v12
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v13, v54
;;                                     v55 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v55
;;                                     v56 = iconst.i32 0
;; @0053                               v14 = icmp eq v50, v56  ; v56 = 0
;; @0053                               brif v14, block5, block2
;;
;;                                 block2:
;; @0053                               v16 = load.i64 notrap aligned v0+56
;; @0053                               v17 = load.i64 notrap aligned v16
;; @0053                               v18 = load.i64 notrap aligned v16+8
;; @0053                               v19 = icmp eq v17, v18
;; @0053                               brif v19, block3, block4
;;
;;                                 block4:
;; @0053                               v21 = load.i64 notrap aligned readonly v0+40
;; @0053                               v22 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v57
;; @0053                               v23 = uextend.i64 v49
;; @0053                               v24 = iconst.i64 8
;; @0053                               v25 = uadd_overflow_trap v23, v24, user65535  ; v24 = 8
;; @0053                               v26 = iconst.i64 8
;; @0053                               v27 = uadd_overflow_trap v25, v26, user65535  ; v26 = 8
;; @0053                               v28 = icmp ult v27, v22
;; @0053                               trapz v28, user65535
;; @0053                               v29 = iadd v21, v25
;; @0053                               v30 = load.i64 notrap aligned v29
;;                                     v58 = iconst.i64 1
;; @0053                               v31 = iadd v30, v58  ; v58 = 1
;; @0053                               v33 = load.i64 notrap aligned readonly v0+40
;; @0053                               v34 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v59
;; @0053                               v35 = uextend.i64 v48
;; @0053                               v36 = iconst.i64 8
;; @0053                               v37 = uadd_overflow_trap v35, v36, user65535  ; v36 = 8
;; @0053                               v38 = iconst.i64 8
;; @0053                               v39 = uadd_overflow_trap v37, v38, user65535  ; v38 = 8
;; @0053                               v40 = icmp ult v39, v34
;; @0053                               trapz v40, user65535
;; @0053                               v41 = iadd v33, v37
;; @0053                               store notrap aligned v31, v41
;;                                     v60 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v60
;; @0053                               store notrap aligned v47, v17
;;                                     v61 = iconst.i64 4
;; @0053                               v42 = iadd.i64 v17, v61  ; v61 = 4
;; @0053                               store notrap aligned v42, v16
;; @0053                               jump block5
;;
;;                                 block3 cold:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v62
;; @0053                               v44 = call fn0(v0, v46), stack_map=[i32 @ ss0+0]
;; @0053                               jump block5
;;
;;                                 block5:
;;                                     v63 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v63
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v45
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+96
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+88
;;                                     v53 = iconst.i64 2
;; @005a                               v9 = ishl v7, v53  ; v53 = 2
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 table_oob aligned table v12
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v13, v54
;;                                     v55 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v55
;;                                     v56 = iconst.i32 0
;; @005a                               v14 = icmp eq v50, v56  ; v56 = 0
;; @005a                               brif v14, block5, block2
;;
;;                                 block2:
;; @005a                               v16 = load.i64 notrap aligned v0+56
;; @005a                               v17 = load.i64 notrap aligned v16
;; @005a                               v18 = load.i64 notrap aligned v16+8
;; @005a                               v19 = icmp eq v17, v18
;; @005a                               brif v19, block3, block4
;;
;;                                 block4:
;; @005a                               v21 = load.i64 notrap aligned readonly v0+40
;; @005a                               v22 = load.i64 notrap aligned readonly v0+48
;;                                     v57 = stack_addr.i64 ss0
;;                                     v49 = load.i32 notrap v57
;; @005a                               v23 = uextend.i64 v49
;; @005a                               v24 = iconst.i64 8
;; @005a                               v25 = uadd_overflow_trap v23, v24, user65535  ; v24 = 8
;; @005a                               v26 = iconst.i64 8
;; @005a                               v27 = uadd_overflow_trap v25, v26, user65535  ; v26 = 8
;; @005a                               v28 = icmp ult v27, v22
;; @005a                               trapz v28, user65535
;; @005a                               v29 = iadd v21, v25
;; @005a                               v30 = load.i64 notrap aligned v29
;;                                     v58 = iconst.i64 1
;; @005a                               v31 = iadd v30, v58  ; v58 = 1
;; @005a                               v33 = load.i64 notrap aligned readonly v0+40
;; @005a                               v34 = load.i64 notrap aligned readonly v0+48
;;                                     v59 = stack_addr.i64 ss0
;;                                     v48 = load.i32 notrap v59
;; @005a                               v35 = uextend.i64 v48
;; @005a                               v36 = iconst.i64 8
;; @005a                               v37 = uadd_overflow_trap v35, v36, user65535  ; v36 = 8
;; @005a                               v38 = iconst.i64 8
;; @005a                               v39 = uadd_overflow_trap v37, v38, user65535  ; v38 = 8
;; @005a                               v40 = icmp ult v39, v34
;; @005a                               trapz v40, user65535
;; @005a                               v41 = iadd v33, v37
;; @005a                               store notrap aligned v31, v41
;;                                     v60 = stack_addr.i64 ss0
;;                                     v47 = load.i32 notrap v60
;; @005a                               store notrap aligned v47, v17
;;                                     v61 = iconst.i64 4
;; @005a                               v42 = iadd.i64 v17, v61  ; v61 = 4
;; @005a                               store notrap aligned v42, v16
;; @005a                               jump block5
;;
;;                                 block3 cold:
;;                                     v62 = stack_addr.i64 ss0
;;                                     v46 = load.i32 notrap v62
;; @005a                               v44 = call fn0(v0, v46), stack_map=[i32 @ ss0+0]
;; @005a                               jump block5
;;
;;                                 block5:
;;                                     v63 = stack_addr.i64 ss0
;;                                     v45 = load.i32 notrap v63
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v45
;; }
