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
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i64 notrap aligned v0+96
;; @0055                               v5 = ireduce.i32 v4
;; @0055                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0055                               v7 = uextend.i64 v3  ; v3 = 0
;; @0055                               v8 = load.i64 notrap aligned v0+88
;;                                     v64 = iconst.i64 2
;; @0055                               v9 = ishl v7, v64  ; v64 = 2
;; @0055                               v10 = iadd v8, v9
;; @0055                               v11 = iconst.i64 0
;; @0055                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0055                               v13 = load.i32 table_oob aligned table v12
;;                                     v65 = iconst.i32 0
;; @0055                               v14 = icmp eq v2, v65  ; v65 = 0
;; @0055                               brif v14, block3, block2
;;
;;                                 block2:
;; @0055                               v16 = load.i64 notrap aligned readonly v0+40
;; @0055                               v17 = load.i64 notrap aligned readonly v0+48
;; @0055                               v18 = uextend.i64 v2
;; @0055                               v19 = iconst.i64 8
;; @0055                               v20 = uadd_overflow_trap v18, v19, user65535  ; v19 = 8
;; @0055                               v21 = iconst.i64 8
;; @0055                               v22 = uadd_overflow_trap v20, v21, user65535  ; v21 = 8
;; @0055                               v23 = icmp ule v22, v17
;; @0055                               trapz v23, user65535
;; @0055                               v24 = iadd v16, v20
;; @0055                               v25 = load.i64 notrap aligned v24
;;                                     v66 = iconst.i64 1
;; @0055                               v26 = iadd v25, v66  ; v66 = 1
;; @0055                               v28 = load.i64 notrap aligned readonly v0+40
;; @0055                               v29 = load.i64 notrap aligned readonly v0+48
;; @0055                               v30 = uextend.i64 v2
;; @0055                               v31 = iconst.i64 8
;; @0055                               v32 = uadd_overflow_trap v30, v31, user65535  ; v31 = 8
;; @0055                               v33 = iconst.i64 8
;; @0055                               v34 = uadd_overflow_trap v32, v33, user65535  ; v33 = 8
;; @0055                               v35 = icmp ule v34, v29
;; @0055                               trapz v35, user65535
;; @0055                               v36 = iadd v28, v32
;; @0055                               store notrap aligned v26, v36
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               store.i32 table_oob aligned table v2, v12
;;                                     v67 = iconst.i32 0
;; @0055                               v37 = icmp.i32 eq v13, v67  ; v67 = 0
;; @0055                               brif v37, block7, block4
;;
;;                                 block4:
;; @0055                               v39 = load.i64 notrap aligned readonly v0+40
;; @0055                               v40 = load.i64 notrap aligned readonly v0+48
;; @0055                               v41 = uextend.i64 v13
;; @0055                               v42 = iconst.i64 8
;; @0055                               v43 = uadd_overflow_trap v41, v42, user65535  ; v42 = 8
;; @0055                               v44 = iconst.i64 8
;; @0055                               v45 = uadd_overflow_trap v43, v44, user65535  ; v44 = 8
;; @0055                               v46 = icmp ule v45, v40
;; @0055                               trapz v46, user65535
;; @0055                               v47 = iadd v39, v43
;; @0055                               v48 = load.i64 notrap aligned v47
;;                                     v68 = iconst.i64 -1
;; @0055                               v49 = iadd v48, v68  ; v68 = -1
;;                                     v69 = iconst.i64 0
;; @0055                               v50 = icmp eq v49, v69  ; v69 = 0
;; @0055                               brif v50, block5, block6
;;
;;                                 block5 cold:
;; @0055                               call fn0(v0, v13)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v53 = load.i64 notrap aligned readonly v0+40
;; @0055                               v54 = load.i64 notrap aligned readonly v0+48
;; @0055                               v55 = uextend.i64 v13
;; @0055                               v56 = iconst.i64 8
;; @0055                               v57 = uadd_overflow_trap v55, v56, user65535  ; v56 = 8
;; @0055                               v58 = iconst.i64 8
;; @0055                               v59 = uadd_overflow_trap v57, v58, user65535  ; v58 = 8
;; @0055                               v60 = icmp ule v59, v54
;; @0055                               trapz v60, user65535
;; @0055                               v61 = iadd v53, v57
;; @0055                               store.i64 notrap aligned v49, v61
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
;;     gv5 = load.i64 notrap aligned gv3+96
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i64 notrap aligned v0+96
;; @005e                               v5 = ireduce.i32 v4
;; @005e                               v6 = icmp uge v2, v5
;; @005e                               v7 = uextend.i64 v2
;; @005e                               v8 = load.i64 notrap aligned v0+88
;;                                     v64 = iconst.i64 2
;; @005e                               v9 = ishl v7, v64  ; v64 = 2
;; @005e                               v10 = iadd v8, v9
;; @005e                               v11 = iconst.i64 0
;; @005e                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005e                               v13 = load.i32 table_oob aligned table v12
;;                                     v65 = iconst.i32 0
;; @005e                               v14 = icmp eq v3, v65  ; v65 = 0
;; @005e                               brif v14, block3, block2
;;
;;                                 block2:
;; @005e                               v16 = load.i64 notrap aligned readonly v0+40
;; @005e                               v17 = load.i64 notrap aligned readonly v0+48
;; @005e                               v18 = uextend.i64 v3
;; @005e                               v19 = iconst.i64 8
;; @005e                               v20 = uadd_overflow_trap v18, v19, user65535  ; v19 = 8
;; @005e                               v21 = iconst.i64 8
;; @005e                               v22 = uadd_overflow_trap v20, v21, user65535  ; v21 = 8
;; @005e                               v23 = icmp ule v22, v17
;; @005e                               trapz v23, user65535
;; @005e                               v24 = iadd v16, v20
;; @005e                               v25 = load.i64 notrap aligned v24
;;                                     v66 = iconst.i64 1
;; @005e                               v26 = iadd v25, v66  ; v66 = 1
;; @005e                               v28 = load.i64 notrap aligned readonly v0+40
;; @005e                               v29 = load.i64 notrap aligned readonly v0+48
;; @005e                               v30 = uextend.i64 v3
;; @005e                               v31 = iconst.i64 8
;; @005e                               v32 = uadd_overflow_trap v30, v31, user65535  ; v31 = 8
;; @005e                               v33 = iconst.i64 8
;; @005e                               v34 = uadd_overflow_trap v32, v33, user65535  ; v33 = 8
;; @005e                               v35 = icmp ule v34, v29
;; @005e                               trapz v35, user65535
;; @005e                               v36 = iadd v28, v32
;; @005e                               store notrap aligned v26, v36
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               store.i32 table_oob aligned table v3, v12
;;                                     v67 = iconst.i32 0
;; @005e                               v37 = icmp.i32 eq v13, v67  ; v67 = 0
;; @005e                               brif v37, block7, block4
;;
;;                                 block4:
;; @005e                               v39 = load.i64 notrap aligned readonly v0+40
;; @005e                               v40 = load.i64 notrap aligned readonly v0+48
;; @005e                               v41 = uextend.i64 v13
;; @005e                               v42 = iconst.i64 8
;; @005e                               v43 = uadd_overflow_trap v41, v42, user65535  ; v42 = 8
;; @005e                               v44 = iconst.i64 8
;; @005e                               v45 = uadd_overflow_trap v43, v44, user65535  ; v44 = 8
;; @005e                               v46 = icmp ule v45, v40
;; @005e                               trapz v46, user65535
;; @005e                               v47 = iadd v39, v43
;; @005e                               v48 = load.i64 notrap aligned v47
;;                                     v68 = iconst.i64 -1
;; @005e                               v49 = iadd v48, v68  ; v68 = -1
;;                                     v69 = iconst.i64 0
;; @005e                               v50 = icmp eq v49, v69  ; v69 = 0
;; @005e                               brif v50, block5, block6
;;
;;                                 block5 cold:
;; @005e                               call fn0(v0, v13)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v53 = load.i64 notrap aligned readonly v0+40
;; @005e                               v54 = load.i64 notrap aligned readonly v0+48
;; @005e                               v55 = uextend.i64 v13
;; @005e                               v56 = iconst.i64 8
;; @005e                               v57 = uadd_overflow_trap v55, v56, user65535  ; v56 = 8
;; @005e                               v58 = iconst.i64 8
;; @005e                               v59 = uadd_overflow_trap v57, v58, user65535  ; v58 = 8
;; @005e                               v60 = icmp ule v59, v54
;; @005e                               trapz v60, user65535
;; @005e                               v61 = iadd v53, v57
;; @005e                               store.i64 notrap aligned v49, v61
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
