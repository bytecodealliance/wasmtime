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
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v62 = iconst.i64 2
;; @0056                               v8 = ishl v6, v62  ; v62 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               v12 = load.i32 user5 aligned table v11
;;                                     v63 = iconst.i32 0
;; @0056                               v13 = icmp eq v2, v63  ; v63 = 0
;; @0056                               brif v13, block3, block2
;;
;;                                 block2:
;; @0056                               v15 = load.i64 notrap aligned readonly v0+40
;; @0056                               v16 = load.i64 notrap aligned readonly v0+48
;; @0056                               v17 = uextend.i64 v2
;; @0056                               v18 = iconst.i64 8
;; @0056                               v19 = uadd_overflow_trap v17, v18, user1  ; v18 = 8
;; @0056                               v20 = iconst.i64 8
;; @0056                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @0056                               v22 = icmp ule v21, v16
;; @0056                               trapz v22, user1
;; @0056                               v23 = iadd v15, v19
;; @0056                               v24 = load.i64 notrap aligned v23
;;                                     v64 = iconst.i64 1
;; @0056                               v25 = iadd v24, v64  ; v64 = 1
;; @0056                               v27 = load.i64 notrap aligned readonly v0+40
;; @0056                               v28 = load.i64 notrap aligned readonly v0+48
;; @0056                               v29 = uextend.i64 v2
;; @0056                               v30 = iconst.i64 8
;; @0056                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @0056                               v32 = iconst.i64 8
;; @0056                               v33 = uadd_overflow_trap v31, v32, user1  ; v32 = 8
;; @0056                               v34 = icmp ule v33, v28
;; @0056                               trapz v34, user1
;; @0056                               v35 = iadd v27, v31
;; @0056                               store notrap aligned v25, v35
;; @0056                               jump block3
;;
;;                                 block3:
;; @0056                               store.i32 user5 aligned table v2, v11
;;                                     v65 = iconst.i32 0
;; @0056                               v36 = icmp.i32 eq v12, v65  ; v65 = 0
;; @0056                               brif v36, block7, block4
;;
;;                                 block4:
;; @0056                               v38 = load.i64 notrap aligned readonly v0+40
;; @0056                               v39 = load.i64 notrap aligned readonly v0+48
;; @0056                               v40 = uextend.i64 v12
;; @0056                               v41 = iconst.i64 8
;; @0056                               v42 = uadd_overflow_trap v40, v41, user1  ; v41 = 8
;; @0056                               v43 = iconst.i64 8
;; @0056                               v44 = uadd_overflow_trap v42, v43, user1  ; v43 = 8
;; @0056                               v45 = icmp ule v44, v39
;; @0056                               trapz v45, user1
;; @0056                               v46 = iadd v38, v42
;; @0056                               v47 = load.i64 notrap aligned v46
;;                                     v66 = iconst.i64 -1
;; @0056                               v48 = iadd v47, v66  ; v66 = -1
;;                                     v67 = iconst.i64 0
;; @0056                               v49 = icmp eq v48, v67  ; v67 = 0
;; @0056                               brif v49, block5, block6
;;
;;                                 block5 cold:
;; @0056                               call fn0(v0, v12)
;; @0056                               jump block7
;;
;;                                 block6:
;; @0056                               v52 = load.i64 notrap aligned readonly v0+40
;; @0056                               v53 = load.i64 notrap aligned readonly v0+48
;; @0056                               v54 = uextend.i64 v12
;; @0056                               v55 = iconst.i64 8
;; @0056                               v56 = uadd_overflow_trap v54, v55, user1  ; v55 = 8
;; @0056                               v57 = iconst.i64 8
;; @0056                               v58 = uadd_overflow_trap v56, v57, user1  ; v57 = 8
;; @0056                               v59 = icmp ule v58, v53
;; @0056                               trapz v59, user1
;; @0056                               v60 = iadd v52, v56
;; @0056                               store.i64 notrap aligned v48, v60
;; @0056                               jump block7
;;
;;                                 block7:
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v62 = iconst.i64 2
;; @005f                               v8 = ishl v6, v62  ; v62 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i32 user5 aligned table v11
;;                                     v63 = iconst.i32 0
;; @005f                               v13 = icmp eq v3, v63  ; v63 = 0
;; @005f                               brif v13, block3, block2
;;
;;                                 block2:
;; @005f                               v15 = load.i64 notrap aligned readonly v0+40
;; @005f                               v16 = load.i64 notrap aligned readonly v0+48
;; @005f                               v17 = uextend.i64 v3
;; @005f                               v18 = iconst.i64 8
;; @005f                               v19 = uadd_overflow_trap v17, v18, user1  ; v18 = 8
;; @005f                               v20 = iconst.i64 8
;; @005f                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @005f                               v22 = icmp ule v21, v16
;; @005f                               trapz v22, user1
;; @005f                               v23 = iadd v15, v19
;; @005f                               v24 = load.i64 notrap aligned v23
;;                                     v64 = iconst.i64 1
;; @005f                               v25 = iadd v24, v64  ; v64 = 1
;; @005f                               v27 = load.i64 notrap aligned readonly v0+40
;; @005f                               v28 = load.i64 notrap aligned readonly v0+48
;; @005f                               v29 = uextend.i64 v3
;; @005f                               v30 = iconst.i64 8
;; @005f                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @005f                               v32 = iconst.i64 8
;; @005f                               v33 = uadd_overflow_trap v31, v32, user1  ; v32 = 8
;; @005f                               v34 = icmp ule v33, v28
;; @005f                               trapz v34, user1
;; @005f                               v35 = iadd v27, v31
;; @005f                               store notrap aligned v25, v35
;; @005f                               jump block3
;;
;;                                 block3:
;; @005f                               store.i32 user5 aligned table v3, v11
;;                                     v65 = iconst.i32 0
;; @005f                               v36 = icmp.i32 eq v12, v65  ; v65 = 0
;; @005f                               brif v36, block7, block4
;;
;;                                 block4:
;; @005f                               v38 = load.i64 notrap aligned readonly v0+40
;; @005f                               v39 = load.i64 notrap aligned readonly v0+48
;; @005f                               v40 = uextend.i64 v12
;; @005f                               v41 = iconst.i64 8
;; @005f                               v42 = uadd_overflow_trap v40, v41, user1  ; v41 = 8
;; @005f                               v43 = iconst.i64 8
;; @005f                               v44 = uadd_overflow_trap v42, v43, user1  ; v43 = 8
;; @005f                               v45 = icmp ule v44, v39
;; @005f                               trapz v45, user1
;; @005f                               v46 = iadd v38, v42
;; @005f                               v47 = load.i64 notrap aligned v46
;;                                     v66 = iconst.i64 -1
;; @005f                               v48 = iadd v47, v66  ; v66 = -1
;;                                     v67 = iconst.i64 0
;; @005f                               v49 = icmp eq v48, v67  ; v67 = 0
;; @005f                               brif v49, block5, block6
;;
;;                                 block5 cold:
;; @005f                               call fn0(v0, v12)
;; @005f                               jump block7
;;
;;                                 block6:
;; @005f                               v52 = load.i64 notrap aligned readonly v0+40
;; @005f                               v53 = load.i64 notrap aligned readonly v0+48
;; @005f                               v54 = uextend.i64 v12
;; @005f                               v55 = iconst.i64 8
;; @005f                               v56 = uadd_overflow_trap v54, v55, user1  ; v55 = 8
;; @005f                               v57 = iconst.i64 8
;; @005f                               v58 = uadd_overflow_trap v56, v57, user1  ; v57 = 8
;; @005f                               v59 = icmp ule v58, v53
;; @005f                               trapz v59, user1
;; @005f                               v60 = iadd v52, v56
;; @005f                               store.i64 notrap aligned v48, v60
;; @005f                               jump block7
;;
;;                                 block7:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
