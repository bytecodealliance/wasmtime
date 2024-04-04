;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimized but with `opt-level=0` to legalize away
;; macro instructions.

(module
  (table (export "table") 7 7 externref)
  (func (export "table.set.const") (param externref)
    i32.const 0
    local.get 0
    table.set 0)
  (func (export "table.set.var") (param i32 externref)
    local.get 0
    local.get 1
    table.set 0))
;; function u0:0(i64 vmctx, i64, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: r64):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v67 = iconst.i64 2
;; @0056                               v8 = ishl v6, v67  ; v67 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               v12 = load.i32 table_oob aligned table v11
;; @0056                               v13 = uextend.i64 v12
;; @0056                               v14 = bitcast.r64 v13
;; @0056                               v15 = is_null v2
;; @0056                               brif v15, block3, block2
;;
;;                                 block2:
;; @0056                               v17 = load.i64 notrap aligned readonly v0+40
;; @0056                               v18 = load.i64 notrap aligned readonly v0+48
;; @0056                               v19 = bitcast.i64 v2
;; @0056                               v20 = iconst.i64 8
;; @0056                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @0056                               v22 = iconst.i64 8
;; @0056                               v23 = uadd_overflow_trap v21, v22, user65535  ; v22 = 8
;; @0056                               v24 = icmp ult v23, v18
;; @0056                               brif v24, block9, block8
;;
;;                                 block8 cold:
;; @0056                               trap user65535
;;
;;                                 block9:
;; @0056                               v25 = iadd.i64 v17, v21
;; @0056                               v26 = load.i64 notrap aligned v25
;;                                     v68 = iconst.i64 1
;; @0056                               v27 = iadd v26, v68  ; v68 = 1
;; @0056                               v29 = load.i64 notrap aligned readonly v0+40
;; @0056                               v30 = load.i64 notrap aligned readonly v0+48
;; @0056                               v31 = bitcast.i64 v2
;; @0056                               v32 = iconst.i64 8
;; @0056                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @0056                               v34 = iconst.i64 8
;; @0056                               v35 = uadd_overflow_trap v33, v34, user65535  ; v34 = 8
;; @0056                               v36 = icmp ult v35, v30
;; @0056                               brif v36, block11, block10
;;
;;                                 block10 cold:
;; @0056                               trap user65535
;;
;;                                 block11:
;; @0056                               v37 = iadd.i64 v29, v33
;; @0056                               store.i64 notrap aligned v27, v37
;; @0056                               jump block3
;;
;;                                 block3:
;; @0056                               v38 = bitcast.i64 v2
;; @0056                               v39 = ireduce.i32 v38
;; @0056                               store table_oob aligned table v39, v11
;; @0056                               v40 = is_null.r64 v14
;; @0056                               brif v40, block7, block4
;;
;;                                 block4:
;; @0056                               v42 = load.i64 notrap aligned readonly v0+40
;; @0056                               v43 = load.i64 notrap aligned readonly v0+48
;; @0056                               v44 = bitcast.i64 v14
;; @0056                               v45 = iconst.i64 8
;; @0056                               v46 = uadd_overflow_trap v44, v45, user65535  ; v45 = 8
;; @0056                               v47 = iconst.i64 8
;; @0056                               v48 = uadd_overflow_trap v46, v47, user65535  ; v47 = 8
;; @0056                               v49 = icmp ult v48, v43
;; @0056                               brif v49, block13, block12
;;
;;                                 block12 cold:
;; @0056                               trap user65535
;;
;;                                 block13:
;; @0056                               v50 = iadd.i64 v42, v46
;; @0056                               v51 = load.i64 notrap aligned v50
;;                                     v69 = iconst.i64 -1
;; @0056                               v52 = iadd v51, v69  ; v69 = -1
;;                                     v70 = iconst.i64 0
;; @0056                               v53 = icmp eq v52, v70  ; v70 = 0
;; @0056                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @0056                               v55 = bitcast.i64 v14
;; @0056                               call fn0(v0, v55)
;; @0056                               jump block7
;;
;;                                 block6:
;; @0056                               v57 = load.i64 notrap aligned readonly v0+40
;; @0056                               v58 = load.i64 notrap aligned readonly v0+48
;; @0056                               v59 = bitcast.i64 v14
;; @0056                               v60 = iconst.i64 8
;; @0056                               v61 = uadd_overflow_trap v59, v60, user65535  ; v60 = 8
;; @0056                               v62 = iconst.i64 8
;; @0056                               v63 = uadd_overflow_trap v61, v62, user65535  ; v62 = 8
;; @0056                               v64 = icmp ult v63, v58
;; @0056                               brif v64, block15, block14
;;
;;                                 block14 cold:
;; @0056                               trap user65535
;;
;;                                 block15:
;; @0056                               v65 = iadd.i64 v57, v61
;; @0056                               store.i64 notrap aligned v52, v65
;; @0056                               jump block7
;;
;;                                 block7:
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: r64):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v67 = iconst.i64 2
;; @005f                               v8 = ishl v6, v67  ; v67 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i32 table_oob aligned table v11
;; @005f                               v13 = uextend.i64 v12
;; @005f                               v14 = bitcast.r64 v13
;; @005f                               v15 = is_null v3
;; @005f                               brif v15, block3, block2
;;
;;                                 block2:
;; @005f                               v17 = load.i64 notrap aligned readonly v0+40
;; @005f                               v18 = load.i64 notrap aligned readonly v0+48
;; @005f                               v19 = bitcast.i64 v3
;; @005f                               v20 = iconst.i64 8
;; @005f                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @005f                               v22 = iconst.i64 8
;; @005f                               v23 = uadd_overflow_trap v21, v22, user65535  ; v22 = 8
;; @005f                               v24 = icmp ult v23, v18
;; @005f                               brif v24, block9, block8
;;
;;                                 block8 cold:
;; @005f                               trap user65535
;;
;;                                 block9:
;; @005f                               v25 = iadd.i64 v17, v21
;; @005f                               v26 = load.i64 notrap aligned v25
;;                                     v68 = iconst.i64 1
;; @005f                               v27 = iadd v26, v68  ; v68 = 1
;; @005f                               v29 = load.i64 notrap aligned readonly v0+40
;; @005f                               v30 = load.i64 notrap aligned readonly v0+48
;; @005f                               v31 = bitcast.i64 v3
;; @005f                               v32 = iconst.i64 8
;; @005f                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @005f                               v34 = iconst.i64 8
;; @005f                               v35 = uadd_overflow_trap v33, v34, user65535  ; v34 = 8
;; @005f                               v36 = icmp ult v35, v30
;; @005f                               brif v36, block11, block10
;;
;;                                 block10 cold:
;; @005f                               trap user65535
;;
;;                                 block11:
;; @005f                               v37 = iadd.i64 v29, v33
;; @005f                               store.i64 notrap aligned v27, v37
;; @005f                               jump block3
;;
;;                                 block3:
;; @005f                               v38 = bitcast.i64 v3
;; @005f                               v39 = ireduce.i32 v38
;; @005f                               store table_oob aligned table v39, v11
;; @005f                               v40 = is_null.r64 v14
;; @005f                               brif v40, block7, block4
;;
;;                                 block4:
;; @005f                               v42 = load.i64 notrap aligned readonly v0+40
;; @005f                               v43 = load.i64 notrap aligned readonly v0+48
;; @005f                               v44 = bitcast.i64 v14
;; @005f                               v45 = iconst.i64 8
;; @005f                               v46 = uadd_overflow_trap v44, v45, user65535  ; v45 = 8
;; @005f                               v47 = iconst.i64 8
;; @005f                               v48 = uadd_overflow_trap v46, v47, user65535  ; v47 = 8
;; @005f                               v49 = icmp ult v48, v43
;; @005f                               brif v49, block13, block12
;;
;;                                 block12 cold:
;; @005f                               trap user65535
;;
;;                                 block13:
;; @005f                               v50 = iadd.i64 v42, v46
;; @005f                               v51 = load.i64 notrap aligned v50
;;                                     v69 = iconst.i64 -1
;; @005f                               v52 = iadd v51, v69  ; v69 = -1
;;                                     v70 = iconst.i64 0
;; @005f                               v53 = icmp eq v52, v70  ; v70 = 0
;; @005f                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @005f                               v55 = bitcast.i64 v14
;; @005f                               call fn0(v0, v55)
;; @005f                               jump block7
;;
;;                                 block6:
;; @005f                               v57 = load.i64 notrap aligned readonly v0+40
;; @005f                               v58 = load.i64 notrap aligned readonly v0+48
;; @005f                               v59 = bitcast.i64 v14
;; @005f                               v60 = iconst.i64 8
;; @005f                               v61 = uadd_overflow_trap v59, v60, user65535  ; v60 = 8
;; @005f                               v62 = iconst.i64 8
;; @005f                               v63 = uadd_overflow_trap v61, v62, user65535  ; v62 = 8
;; @005f                               v64 = icmp ult v63, v58
;; @005f                               brif v64, block15, block14
;;
;;                                 block14 cold:
;; @005f                               trap user65535
;;
;;                                 block15:
;; @005f                               v65 = iadd.i64 v57, v61
;; @005f                               store.i64 notrap aligned v52, v65
;; @005f                               jump block7
;;
;;                                 block7:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
