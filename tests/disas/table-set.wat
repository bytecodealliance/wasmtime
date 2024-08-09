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

;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i32 notrap aligned v0+96
;; @0055                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0055                               v6 = uextend.i64 v3  ; v3 = 0
;; @0055                               v7 = load.i64 notrap aligned v0+88
;;                                     v63 = iconst.i64 2
;; @0055                               v8 = ishl v6, v63  ; v63 = 2
;; @0055                               v9 = iadd v7, v8
;; @0055                               v10 = iconst.i64 0
;; @0055                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0055                               v12 = load.i32 table_oob aligned table v11
;;                                     v64 = iconst.i32 0
;; @0055                               v13 = icmp eq v2, v64  ; v64 = 0
;; @0055                               brif v13, block3, block2
;;
;;                                 block2:
;; @0055                               v15 = load.i64 notrap aligned readonly v0+40
;; @0055                               v16 = load.i64 notrap aligned readonly v0+48
;; @0055                               v17 = uextend.i64 v2
;; @0055                               v18 = iconst.i64 8
;; @0055                               v19 = uadd_overflow_trap v17, v18, user65535  ; v18 = 8
;; @0055                               v20 = iconst.i64 8
;; @0055                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @0055                               v22 = icmp ult v21, v16
;; @0055                               brif v22, block9, block8
;;
;;                                 block8 cold:
;; @0055                               trap user65535
;;
;;                                 block9:
;; @0055                               v23 = iadd.i64 v15, v19
;; @0055                               v24 = load.i64 notrap aligned v23
;;                                     v65 = iconst.i64 1
;; @0055                               v25 = iadd v24, v65  ; v65 = 1
;; @0055                               v27 = load.i64 notrap aligned readonly v0+40
;; @0055                               v28 = load.i64 notrap aligned readonly v0+48
;; @0055                               v29 = uextend.i64 v2
;; @0055                               v30 = iconst.i64 8
;; @0055                               v31 = uadd_overflow_trap v29, v30, user65535  ; v30 = 8
;; @0055                               v32 = iconst.i64 8
;; @0055                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @0055                               v34 = icmp ult v33, v28
;; @0055                               brif v34, block11, block10
;;
;;                                 block10 cold:
;; @0055                               trap user65535
;;
;;                                 block11:
;; @0055                               v35 = iadd.i64 v27, v31
;; @0055                               store.i64 notrap aligned v25, v35
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               store.i32 table_oob aligned table v2, v11
;;                                     v66 = iconst.i32 0
;; @0055                               v36 = icmp.i32 eq v12, v66  ; v66 = 0
;; @0055                               brif v36, block7, block4
;;
;;                                 block4:
;; @0055                               v38 = load.i64 notrap aligned readonly v0+40
;; @0055                               v39 = load.i64 notrap aligned readonly v0+48
;; @0055                               v40 = uextend.i64 v12
;; @0055                               v41 = iconst.i64 8
;; @0055                               v42 = uadd_overflow_trap v40, v41, user65535  ; v41 = 8
;; @0055                               v43 = iconst.i64 8
;; @0055                               v44 = uadd_overflow_trap v42, v43, user65535  ; v43 = 8
;; @0055                               v45 = icmp ult v44, v39
;; @0055                               brif v45, block13, block12
;;
;;                                 block12 cold:
;; @0055                               trap user65535
;;
;;                                 block13:
;; @0055                               v46 = iadd.i64 v38, v42
;; @0055                               v47 = load.i64 notrap aligned v46
;;                                     v67 = iconst.i64 -1
;; @0055                               v48 = iadd v47, v67  ; v67 = -1
;;                                     v68 = iconst.i64 0
;; @0055                               v49 = icmp eq v48, v68  ; v68 = 0
;; @0055                               brif v49, block5, block6
;;
;;                                 block5 cold:
;; @0055                               call fn0(v0, v12)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v52 = load.i64 notrap aligned readonly v0+40
;; @0055                               v53 = load.i64 notrap aligned readonly v0+48
;; @0055                               v54 = uextend.i64 v12
;; @0055                               v55 = iconst.i64 8
;; @0055                               v56 = uadd_overflow_trap v54, v55, user65535  ; v55 = 8
;; @0055                               v57 = iconst.i64 8
;; @0055                               v58 = uadd_overflow_trap v56, v57, user65535  ; v57 = 8
;; @0055                               v59 = icmp ult v58, v53
;; @0055                               brif v59, block15, block14
;;
;;                                 block14 cold:
;; @0055                               trap user65535
;;
;;                                 block15:
;; @0055                               v60 = iadd.i64 v52, v56
;; @0055                               store.i64 notrap aligned v48, v60
;; @0055                               jump block7
;;
;;                                 block7:
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+88
;;     gv5 = load.i32 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i32 notrap aligned v0+96
;; @005e                               v5 = icmp uge v2, v4
;; @005e                               v6 = uextend.i64 v2
;; @005e                               v7 = load.i64 notrap aligned v0+88
;;                                     v63 = iconst.i64 2
;; @005e                               v8 = ishl v6, v63  ; v63 = 2
;; @005e                               v9 = iadd v7, v8
;; @005e                               v10 = iconst.i64 0
;; @005e                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005e                               v12 = load.i32 table_oob aligned table v11
;;                                     v64 = iconst.i32 0
;; @005e                               v13 = icmp eq v3, v64  ; v64 = 0
;; @005e                               brif v13, block3, block2
;;
;;                                 block2:
;; @005e                               v15 = load.i64 notrap aligned readonly v0+40
;; @005e                               v16 = load.i64 notrap aligned readonly v0+48
;; @005e                               v17 = uextend.i64 v3
;; @005e                               v18 = iconst.i64 8
;; @005e                               v19 = uadd_overflow_trap v17, v18, user65535  ; v18 = 8
;; @005e                               v20 = iconst.i64 8
;; @005e                               v21 = uadd_overflow_trap v19, v20, user65535  ; v20 = 8
;; @005e                               v22 = icmp ult v21, v16
;; @005e                               brif v22, block9, block8
;;
;;                                 block8 cold:
;; @005e                               trap user65535
;;
;;                                 block9:
;; @005e                               v23 = iadd.i64 v15, v19
;; @005e                               v24 = load.i64 notrap aligned v23
;;                                     v65 = iconst.i64 1
;; @005e                               v25 = iadd v24, v65  ; v65 = 1
;; @005e                               v27 = load.i64 notrap aligned readonly v0+40
;; @005e                               v28 = load.i64 notrap aligned readonly v0+48
;; @005e                               v29 = uextend.i64 v3
;; @005e                               v30 = iconst.i64 8
;; @005e                               v31 = uadd_overflow_trap v29, v30, user65535  ; v30 = 8
;; @005e                               v32 = iconst.i64 8
;; @005e                               v33 = uadd_overflow_trap v31, v32, user65535  ; v32 = 8
;; @005e                               v34 = icmp ult v33, v28
;; @005e                               brif v34, block11, block10
;;
;;                                 block10 cold:
;; @005e                               trap user65535
;;
;;                                 block11:
;; @005e                               v35 = iadd.i64 v27, v31
;; @005e                               store.i64 notrap aligned v25, v35
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               store.i32 table_oob aligned table v3, v11
;;                                     v66 = iconst.i32 0
;; @005e                               v36 = icmp.i32 eq v12, v66  ; v66 = 0
;; @005e                               brif v36, block7, block4
;;
;;                                 block4:
;; @005e                               v38 = load.i64 notrap aligned readonly v0+40
;; @005e                               v39 = load.i64 notrap aligned readonly v0+48
;; @005e                               v40 = uextend.i64 v12
;; @005e                               v41 = iconst.i64 8
;; @005e                               v42 = uadd_overflow_trap v40, v41, user65535  ; v41 = 8
;; @005e                               v43 = iconst.i64 8
;; @005e                               v44 = uadd_overflow_trap v42, v43, user65535  ; v43 = 8
;; @005e                               v45 = icmp ult v44, v39
;; @005e                               brif v45, block13, block12
;;
;;                                 block12 cold:
;; @005e                               trap user65535
;;
;;                                 block13:
;; @005e                               v46 = iadd.i64 v38, v42
;; @005e                               v47 = load.i64 notrap aligned v46
;;                                     v67 = iconst.i64 -1
;; @005e                               v48 = iadd v47, v67  ; v67 = -1
;;                                     v68 = iconst.i64 0
;; @005e                               v49 = icmp eq v48, v68  ; v68 = 0
;; @005e                               brif v49, block5, block6
;;
;;                                 block5 cold:
;; @005e                               call fn0(v0, v12)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v52 = load.i64 notrap aligned readonly v0+40
;; @005e                               v53 = load.i64 notrap aligned readonly v0+48
;; @005e                               v54 = uextend.i64 v12
;; @005e                               v55 = iconst.i64 8
;; @005e                               v56 = uadd_overflow_trap v54, v55, user65535  ; v55 = 8
;; @005e                               v57 = iconst.i64 8
;; @005e                               v58 = uadd_overflow_trap v56, v57, user65535  ; v57 = 8
;; @005e                               v59 = icmp ult v58, v53
;; @005e                               brif v59, block15, block14
;;
;;                                 block14 cold:
;; @005e                               trap user65535
;;
;;                                 block15:
;; @005e                               v60 = iadd.i64 v52, v56
;; @005e                               store.i64 notrap aligned v48, v60
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
