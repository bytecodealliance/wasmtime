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

;; function u0:0(i64 vmctx, i64) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, r64) -> r64 system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v16 -> v0
;;                                     v21 -> v0
;;                                     v33 -> v0
;;                                     v44 -> v0
;;                                     v48 -> v0
;;                                     v49 -> v0
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i32 notrap aligned v0+96
;; @0053                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0053                               v6 = uextend.i64 v3  ; v3 = 0
;; @0053                               v7 = load.i64 notrap aligned v0+88
;;                                     v50 = iconst.i64 3
;; @0053                               v8 = ishl v6, v50  ; v50 = 3
;; @0053                               v9 = iadd v7, v8
;; @0053                               v10 = iconst.i64 0
;; @0053                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0053                               v12 = load.i32 table_oob aligned table v11
;; @0053                               v13 = uextend.i64 v12
;; @0053                               v14 = bitcast.r64 v13
;;                                     v2 -> v14
;; @0053                               v15 = is_null v14
;; @0053                               brif v15, block5, block2
;;
;;                                 block2:
;; @0053                               v17 = load.i64 notrap aligned v0+48
;; @0053                               v18 = load.i64 notrap aligned v17
;; @0053                               v19 = load.i64 notrap aligned v17+8
;; @0053                               v20 = icmp eq v18, v19
;; @0053                               brif v20, block3, block4
;;
;;                                 block4:
;; @0053                               v22 = load.i64 notrap aligned readonly v0+32
;; @0053                               v23 = load.i64 notrap aligned readonly v0+40
;; @0053                               v24 = bitcast.i64 v14
;; @0053                               v25 = iconst.i64 8
;; @0053                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @0053                               v27 = iconst.i64 8
;; @0053                               v28 = uadd_overflow_trap v26, v27, user65535  ; v27 = 8
;; @0053                               v29 = icmp ult v28, v23
;; @0053                               brif v29, block7, block6
;;
;;                                 block6 cold:
;; @0053                               trap user65535
;;
;;                                 block7:
;; @0053                               v30 = iadd.i64 v22, v26
;; @0053                               v31 = load.i64 notrap aligned v30
;;                                     v51 = iconst.i64 1
;; @0053                               v32 = iadd v31, v51  ; v51 = 1
;; @0053                               v34 = load.i64 notrap aligned readonly v0+32
;; @0053                               v35 = load.i64 notrap aligned readonly v0+40
;; @0053                               v36 = bitcast.i64 v14
;; @0053                               v37 = iconst.i64 8
;; @0053                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @0053                               v39 = iconst.i64 8
;; @0053                               v40 = uadd_overflow_trap v38, v39, user65535  ; v39 = 8
;; @0053                               v41 = icmp ult v40, v35
;; @0053                               brif v41, block9, block8
;;
;;                                 block8 cold:
;; @0053                               trap user65535
;;
;;                                 block9:
;; @0053                               v42 = iadd.i64 v34, v38
;; @0053                               store.i64 notrap aligned v32, v42
;; @0053                               store.r64 notrap aligned v14, v18
;;                                     v52 = iconst.i64 8
;; @0053                               v43 = iadd.i64 v18, v52  ; v52 = 8
;; @0053                               store notrap aligned v43, v17
;; @0053                               jump block5
;;
;;                                 block3 cold:
;; @0053                               v45 = load.i64 notrap aligned readonly v0+72
;; @0053                               v46 = load.i64 notrap aligned readonly v45+208
;; @0053                               v47 = call_indirect sig0, v46(v0, v14)
;; @0053                               jump block5
;;
;;                                 block5:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v14
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, r64) -> r64 system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v16 -> v0
;;                                     v21 -> v0
;;                                     v33 -> v0
;;                                     v44 -> v0
;;                                     v48 -> v0
;;                                     v49 -> v0
;; @005a                               v4 = load.i32 notrap aligned v0+96
;; @005a                               v5 = icmp uge v2, v4
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v7 = load.i64 notrap aligned v0+88
;;                                     v50 = iconst.i64 3
;; @005a                               v8 = ishl v6, v50  ; v50 = 3
;; @005a                               v9 = iadd v7, v8
;; @005a                               v10 = iconst.i64 0
;; @005a                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005a                               v12 = load.i32 table_oob aligned table v11
;; @005a                               v13 = uextend.i64 v12
;; @005a                               v14 = bitcast.r64 v13
;;                                     v3 -> v14
;; @005a                               v15 = is_null v14
;; @005a                               brif v15, block5, block2
;;
;;                                 block2:
;; @005a                               v17 = load.i64 notrap aligned v0+48
;; @005a                               v18 = load.i64 notrap aligned v17
;; @005a                               v19 = load.i64 notrap aligned v17+8
;; @005a                               v20 = icmp eq v18, v19
;; @005a                               brif v20, block3, block4
;;
;;                                 block4:
;; @005a                               v22 = load.i64 notrap aligned readonly v0+32
;; @005a                               v23 = load.i64 notrap aligned readonly v0+40
;; @005a                               v24 = bitcast.i64 v14
;; @005a                               v25 = iconst.i64 8
;; @005a                               v26 = uadd_overflow_trap v24, v25, user65535  ; v25 = 8
;; @005a                               v27 = iconst.i64 8
;; @005a                               v28 = uadd_overflow_trap v26, v27, user65535  ; v27 = 8
;; @005a                               v29 = icmp ult v28, v23
;; @005a                               brif v29, block7, block6
;;
;;                                 block6 cold:
;; @005a                               trap user65535
;;
;;                                 block7:
;; @005a                               v30 = iadd.i64 v22, v26
;; @005a                               v31 = load.i64 notrap aligned v30
;;                                     v51 = iconst.i64 1
;; @005a                               v32 = iadd v31, v51  ; v51 = 1
;; @005a                               v34 = load.i64 notrap aligned readonly v0+32
;; @005a                               v35 = load.i64 notrap aligned readonly v0+40
;; @005a                               v36 = bitcast.i64 v14
;; @005a                               v37 = iconst.i64 8
;; @005a                               v38 = uadd_overflow_trap v36, v37, user65535  ; v37 = 8
;; @005a                               v39 = iconst.i64 8
;; @005a                               v40 = uadd_overflow_trap v38, v39, user65535  ; v39 = 8
;; @005a                               v41 = icmp ult v40, v35
;; @005a                               brif v41, block9, block8
;;
;;                                 block8 cold:
;; @005a                               trap user65535
;;
;;                                 block9:
;; @005a                               v42 = iadd.i64 v34, v38
;; @005a                               store.i64 notrap aligned v32, v42
;; @005a                               store.r64 notrap aligned v14, v18
;;                                     v52 = iconst.i64 8
;; @005a                               v43 = iadd.i64 v18, v52  ; v52 = 8
;; @005a                               store notrap aligned v43, v17
;; @005a                               jump block5
;;
;;                                 block3 cold:
;; @005a                               v45 = load.i64 notrap aligned readonly v0+72
;; @005a                               v46 = load.i64 notrap aligned readonly v45+208
;; @005a                               v47 = call_indirect sig0, v46(v0, v14)
;; @005a                               jump block5
;;
;;                                 block5:
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v14
;; }
