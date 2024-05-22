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

;; function u0:0(i64 vmctx, i64) -> r64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, r64) -> r64 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v47 = iconst.i64 2
;; @0054                               v8 = ishl v6, v47  ; v47 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 table_oob aligned table v11
;; @0054                               v13 = uextend.i64 v12
;; @0054                               v14 = bitcast.r64 v13
;; @0054                               v15 = is_null v14
;; @0054                               brif v15, block5, block2
;;
;;                                 block2:
;; @0054                               v17 = load.i64 notrap aligned v0+56
;; @0054                               v18 = load.i64 notrap aligned v17
;; @0054                               v19 = load.i64 notrap aligned v17+8
;; @0054                               v20 = icmp eq v18, v19
;; @0054                               brif v20, block3, block4
;;
;;                                 block4:
;; @0054                               v22 = load.i64 notrap aligned readonly v0+40
;; @0054                               v23 = load.i64 notrap aligned readonly v0+48
;; @0054                               v24 = bitcast.i64 v14
;; @0054                               v25 = iconst.i64 8
;; @0054                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @0054                               v27 = iconst.i64 8
;; @0054                               v28 = uadd_overflow_trap v26, v27, user65535  ; v27 = 8
;; @0054                               v29 = icmp ult v28, v23
;; @0054                               brif v29, block7, block6
;;
;;                                 block6 cold:
;; @0054                               trap user65535
;;
;;                                 block7:
;; @0054                               v30 = iadd.i64 v22, v26
;; @0054                               v31 = load.i64 notrap aligned v30
;;                                     v48 = iconst.i64 1
;; @0054                               v32 = iadd v31, v48  ; v48 = 1
;; @0054                               v34 = load.i64 notrap aligned readonly v0+40
;; @0054                               v35 = load.i64 notrap aligned readonly v0+48
;; @0054                               v36 = bitcast.i64 v14
;; @0054                               v37 = iconst.i64 8
;; @0054                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @0054                               v39 = iconst.i64 8
;; @0054                               v40 = uadd_overflow_trap v38, v39, user65535  ; v39 = 8
;; @0054                               v41 = icmp ult v40, v35
;; @0054                               brif v41, block9, block8
;;
;;                                 block8 cold:
;; @0054                               trap user65535
;;
;;                                 block9:
;; @0054                               v42 = iadd.i64 v34, v38
;; @0054                               store.i64 notrap aligned v32, v42
;; @0054                               store.r64 notrap aligned v14, v18
;;                                     v49 = iconst.i64 8
;; @0054                               v43 = iadd.i64 v18, v49  ; v49 = 8
;; @0054                               store notrap aligned v43, v17
;; @0054                               jump block5
;;
;;                                 block3 cold:
;; @0054                               v45 = call fn0(v0, v14)
;; @0054                               jump block5
;;
;;                                 block5:
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v14
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> r64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, r64) -> r64 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v47 = iconst.i64 2
;; @005b                               v8 = ishl v6, v47  ; v47 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 table_oob aligned table v11
;; @005b                               v13 = uextend.i64 v12
;; @005b                               v14 = bitcast.r64 v13
;; @005b                               v15 = is_null v14
;; @005b                               brif v15, block5, block2
;;
;;                                 block2:
;; @005b                               v17 = load.i64 notrap aligned v0+56
;; @005b                               v18 = load.i64 notrap aligned v17
;; @005b                               v19 = load.i64 notrap aligned v17+8
;; @005b                               v20 = icmp eq v18, v19
;; @005b                               brif v20, block3, block4
;;
;;                                 block4:
;; @005b                               v22 = load.i64 notrap aligned readonly v0+40
;; @005b                               v23 = load.i64 notrap aligned readonly v0+48
;; @005b                               v24 = bitcast.i64 v14
;; @005b                               v25 = iconst.i64 8
;; @005b                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @005b                               v27 = iconst.i64 8
;; @005b                               v28 = uadd_overflow_trap v26, v27, user65535  ; v27 = 8
;; @005b                               v29 = icmp ult v28, v23
;; @005b                               brif v29, block7, block6
;;
;;                                 block6 cold:
;; @005b                               trap user65535
;;
;;                                 block7:
;; @005b                               v30 = iadd.i64 v22, v26
;; @005b                               v31 = load.i64 notrap aligned v30
;;                                     v48 = iconst.i64 1
;; @005b                               v32 = iadd v31, v48  ; v48 = 1
;; @005b                               v34 = load.i64 notrap aligned readonly v0+40
;; @005b                               v35 = load.i64 notrap aligned readonly v0+48
;; @005b                               v36 = bitcast.i64 v14
;; @005b                               v37 = iconst.i64 8
;; @005b                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @005b                               v39 = iconst.i64 8
;; @005b                               v40 = uadd_overflow_trap v38, v39, user65535  ; v39 = 8
;; @005b                               v41 = icmp ult v40, v35
;; @005b                               brif v41, block9, block8
;;
;;                                 block8 cold:
;; @005b                               trap user65535
;;
;;                                 block9:
;; @005b                               v42 = iadd.i64 v34, v38
;; @005b                               store.i64 notrap aligned v32, v42
;; @005b                               store.r64 notrap aligned v14, v18
;;                                     v49 = iconst.i64 8
;; @005b                               v43 = iadd.i64 v18, v49  ; v49 = 8
;; @005b                               store notrap aligned v43, v17
;; @005b                               jump block5
;;
;;                                 block3 cold:
;; @005b                               v45 = call fn0(v0, v14)
;; @005b                               jump block5
;;
;;                                 block5:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v14
;; }
