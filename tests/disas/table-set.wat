;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with `opt-level=0` to legalize away table_addr instructions.

(module
  (table (export "table") 1 externref)
  (func (export "table.set.const") (param externref)
    i32.const 0
    local.get 0
    table.set 0)
  (func (export "table.set.var") (param i32 externref)
    local.get 0
    local.get 1
    table.set 0))

;; function u0:0(i64 vmctx, i64, r64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i64) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: r64):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i32 notrap aligned v0+96
;; @0055                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0055                               v6 = uextend.i64 v3  ; v3 = 0
;; @0055                               v7 = load.i64 notrap aligned v0+88
;;                                     v68 = iconst.i64 2
;; @0055                               v8 = ishl v6, v68  ; v68 = 2
;; @0055                               v9 = iadd v7, v8
;; @0055                               v10 = iconst.i64 0
;; @0055                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0055                               v12 = load.i32 table_oob aligned table v11
;; @0055                               v13 = uextend.i64 v12
;; @0055                               v14 = bitcast.r64 v13
;; @0055                               v15 = is_null v2
;; @0055                               brif v15, block3, block2
;;
;;                                 block2:
;; @0055                               v17 = load.i64 notrap aligned readonly v0+40
;; @0055                               v18 = load.i64 notrap aligned readonly v0+48
;; @0055                               v19 = bitcast.i64 v2
;; @0055                               v20 = iconst.i64 8
;; @0055                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @0055                               v22 = iconst.i64 8
;; @0055                               v23 = uadd_overflow_trap v21, v22, user65535  ; v22 = 8
;; @0055                               v24 = icmp ult v23, v18
;; @0055                               brif v24, block9, block8
;;
;;                                 block8 cold:
;; @0055                               trap user65535
;;
;;                                 block9:
;; @0055                               v25 = iadd.i64 v17, v21
;; @0055                               v26 = load.i64 notrap aligned v25
;;                                     v69 = iconst.i64 1
;; @0055                               v27 = iadd v26, v69  ; v69 = 1
;; @0055                               v29 = load.i64 notrap aligned readonly v0+40
;; @0055                               v30 = load.i64 notrap aligned readonly v0+48
;; @0055                               v31 = bitcast.i64 v2
;; @0055                               v32 = iconst.i64 8
;; @0055                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @0055                               v34 = iconst.i64 8
;; @0055                               v35 = uadd_overflow_trap v33, v34, user65535  ; v34 = 8
;; @0055                               v36 = icmp ult v35, v30
;; @0055                               brif v36, block11, block10
;;
;;                                 block10 cold:
;; @0055                               trap user65535
;;
;;                                 block11:
;; @0055                               v37 = iadd.i64 v29, v33
;; @0055                               store.i64 notrap aligned v27, v37
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               v38 = bitcast.i64 v2
;; @0055                               v39 = ireduce.i32 v38
;; @0055                               store table_oob aligned table v39, v11
;; @0055                               v40 = is_null.r64 v14
;; @0055                               brif v40, block7, block4
;;
;;                                 block4:
;; @0055                               v42 = load.i64 notrap aligned readonly v0+40
;; @0055                               v43 = load.i64 notrap aligned readonly v0+48
;; @0055                               v44 = bitcast.i64 v14
;; @0055                               v45 = iconst.i64 8
;; @0055                               v46 = uadd_overflow_trap v44, v45, user65535  ; v45 = 8
;; @0055                               v47 = iconst.i64 8
;; @0055                               v48 = uadd_overflow_trap v46, v47, user65535  ; v47 = 8
;; @0055                               v49 = icmp ult v48, v43
;; @0055                               brif v49, block13, block12
;;
;;                                 block12 cold:
;; @0055                               trap user65535
;;
;;                                 block13:
;; @0055                               v50 = iadd.i64 v42, v46
;; @0055                               v51 = load.i64 notrap aligned v50
;;                                     v70 = iconst.i64 -1
;; @0055                               v52 = iadd v51, v70  ; v70 = -1
;;                                     v71 = iconst.i64 0
;; @0055                               v53 = icmp eq v52, v71  ; v71 = 0
;; @0055                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @0055                               v55 = bitcast.i64 v14
;; @0055                               call fn0(v0, v55)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v57 = load.i64 notrap aligned readonly v0+40
;; @0055                               v58 = load.i64 notrap aligned readonly v0+48
;; @0055                               v59 = bitcast.i64 v14
;; @0055                               v60 = iconst.i64 8
;; @0055                               v61 = uadd_overflow_trap v59, v60, user65535  ; v60 = 8
;; @0055                               v62 = iconst.i64 8
;; @0055                               v63 = uadd_overflow_trap v61, v62, user65535  ; v62 = 8
;; @0055                               v64 = icmp ult v63, v58
;; @0055                               brif v64, block15, block14
;;
;;                                 block14 cold:
;; @0055                               trap user65535
;;
;;                                 block15:
;; @0055                               v65 = iadd.i64 v57, v61
;; @0055                               store.i64 notrap aligned v52, v65
;; @0055                               jump block7
;;
;;                                 block7:
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, r64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i64) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: r64):
;; @005e                               v4 = load.i32 notrap aligned v0+96
;; @005e                               v5 = icmp uge v2, v4
;; @005e                               v6 = uextend.i64 v2
;; @005e                               v7 = load.i64 notrap aligned v0+88
;;                                     v68 = iconst.i64 2
;; @005e                               v8 = ishl v6, v68  ; v68 = 2
;; @005e                               v9 = iadd v7, v8
;; @005e                               v10 = iconst.i64 0
;; @005e                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005e                               v12 = load.i32 table_oob aligned table v11
;; @005e                               v13 = uextend.i64 v12
;; @005e                               v14 = bitcast.r64 v13
;; @005e                               v15 = is_null v3
;; @005e                               brif v15, block3, block2
;;
;;                                 block2:
;; @005e                               v17 = load.i64 notrap aligned readonly v0+40
;; @005e                               v18 = load.i64 notrap aligned readonly v0+48
;; @005e                               v19 = bitcast.i64 v3
;; @005e                               v20 = iconst.i64 8
;; @005e                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @005e                               v22 = iconst.i64 8
;; @005e                               v23 = uadd_overflow_trap v21, v22, user65535  ; v22 = 8
;; @005e                               v24 = icmp ult v23, v18
;; @005e                               brif v24, block9, block8
;;
;;                                 block8 cold:
;; @005e                               trap user65535
;;
;;                                 block9:
;; @005e                               v25 = iadd.i64 v17, v21
;; @005e                               v26 = load.i64 notrap aligned v25
;;                                     v69 = iconst.i64 1
;; @005e                               v27 = iadd v26, v69  ; v69 = 1
;; @005e                               v29 = load.i64 notrap aligned readonly v0+40
;; @005e                               v30 = load.i64 notrap aligned readonly v0+48
;; @005e                               v31 = bitcast.i64 v3
;; @005e                               v32 = iconst.i64 8
;; @005e                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @005e                               v34 = iconst.i64 8
;; @005e                               v35 = uadd_overflow_trap v33, v34, user65535  ; v34 = 8
;; @005e                               v36 = icmp ult v35, v30
;; @005e                               brif v36, block11, block10
;;
;;                                 block10 cold:
;; @005e                               trap user65535
;;
;;                                 block11:
;; @005e                               v37 = iadd.i64 v29, v33
;; @005e                               store.i64 notrap aligned v27, v37
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               v38 = bitcast.i64 v3
;; @005e                               v39 = ireduce.i32 v38
;; @005e                               store table_oob aligned table v39, v11
;; @005e                               v40 = is_null.r64 v14
;; @005e                               brif v40, block7, block4
;;
;;                                 block4:
;; @005e                               v42 = load.i64 notrap aligned readonly v0+40
;; @005e                               v43 = load.i64 notrap aligned readonly v0+48
;; @005e                               v44 = bitcast.i64 v14
;; @005e                               v45 = iconst.i64 8
;; @005e                               v46 = uadd_overflow_trap v44, v45, user65535  ; v45 = 8
;; @005e                               v47 = iconst.i64 8
;; @005e                               v48 = uadd_overflow_trap v46, v47, user65535  ; v47 = 8
;; @005e                               v49 = icmp ult v48, v43
;; @005e                               brif v49, block13, block12
;;
;;                                 block12 cold:
;; @005e                               trap user65535
;;
;;                                 block13:
;; @005e                               v50 = iadd.i64 v42, v46
;; @005e                               v51 = load.i64 notrap aligned v50
;;                                     v70 = iconst.i64 -1
;; @005e                               v52 = iadd v51, v70  ; v70 = -1
;;                                     v71 = iconst.i64 0
;; @005e                               v53 = icmp eq v52, v71  ; v71 = 0
;; @005e                               brif v53, block5, block6
;;
;;                                 block5 cold:
;; @005e                               v55 = bitcast.i64 v14
;; @005e                               call fn0(v0, v55)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v57 = load.i64 notrap aligned readonly v0+40
;; @005e                               v58 = load.i64 notrap aligned readonly v0+48
;; @005e                               v59 = bitcast.i64 v14
;; @005e                               v60 = iconst.i64 8
;; @005e                               v61 = uadd_overflow_trap v59, v60, user65535  ; v60 = 8
;; @005e                               v62 = iconst.i64 8
;; @005e                               v63 = uadd_overflow_trap v61, v62, user65535  ; v62 = 8
;; @005e                               v64 = icmp ult v63, v58
;; @005e                               brif v64, block15, block14
;;
;;                                 block14 cold:
;; @005e                               trap user65535
;;
;;                                 block15:
;; @005e                               v65 = iadd.i64 v57, v61
;; @005e                               store.i64 notrap aligned v52, v65
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
