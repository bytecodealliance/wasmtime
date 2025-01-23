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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v66 = iconst.i64 2
;; @0056                               v8 = ishl v6, v66  ; v66 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               v12 = load.i32 user5 aligned table v11
;;                                     v67 = iconst.i32 0
;; @0056                               v13 = icmp eq v2, v67  ; v67 = 0
;; @0056                               brif v13, block3, block2
;;
;;                                 block2:
;; @0056                               v15 = load.i64 notrap aligned readonly v0+40
;; @0056                               v17 = load.i64 notrap aligned readonly v0+48
;; @0056                               v18 = uextend.i64 v2
;; @0056                               v19 = iconst.i64 8
;; @0056                               v20 = uadd_overflow_trap v18, v19, user1  ; v19 = 8
;; @0056                               v21 = iconst.i64 8
;; @0056                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 8
;; @0056                               v23 = icmp ule v22, v17
;; @0056                               trapz v23, user1
;; @0056                               v24 = iadd v15, v20
;; @0056                               v25 = load.i64 notrap aligned v24
;;                                     v68 = iconst.i64 1
;; @0056                               v26 = iadd v25, v68  ; v68 = 1
;; @0056                               v28 = load.i64 notrap aligned readonly v0+40
;; @0056                               v30 = load.i64 notrap aligned readonly v0+48
;; @0056                               v31 = uextend.i64 v2
;; @0056                               v32 = iconst.i64 8
;; @0056                               v33 = uadd_overflow_trap v31, v32, user1  ; v32 = 8
;; @0056                               v34 = iconst.i64 8
;; @0056                               v35 = uadd_overflow_trap v33, v34, user1  ; v34 = 8
;; @0056                               v36 = icmp ule v35, v30
;; @0056                               trapz v36, user1
;; @0056                               v37 = iadd v28, v33
;; @0056                               store notrap aligned v26, v37
;; @0056                               jump block3
;;
;;                                 block3:
;; @0056                               store.i32 user5 aligned table v2, v11
;;                                     v69 = iconst.i32 0
;; @0056                               v38 = icmp.i32 eq v12, v69  ; v69 = 0
;; @0056                               brif v38, block7, block4
;;
;;                                 block4:
;; @0056                               v40 = load.i64 notrap aligned readonly v0+40
;; @0056                               v42 = load.i64 notrap aligned readonly v0+48
;; @0056                               v43 = uextend.i64 v12
;; @0056                               v44 = iconst.i64 8
;; @0056                               v45 = uadd_overflow_trap v43, v44, user1  ; v44 = 8
;; @0056                               v46 = iconst.i64 8
;; @0056                               v47 = uadd_overflow_trap v45, v46, user1  ; v46 = 8
;; @0056                               v48 = icmp ule v47, v42
;; @0056                               trapz v48, user1
;; @0056                               v49 = iadd v40, v45
;; @0056                               v50 = load.i64 notrap aligned v49
;;                                     v70 = iconst.i64 -1
;; @0056                               v51 = iadd v50, v70  ; v70 = -1
;;                                     v71 = iconst.i64 0
;; @0056                               v52 = icmp eq v51, v71  ; v71 = 0
;; @0056                               brif v52, block5, block6
;;
;;                                 block5 cold:
;; @0056                               call fn0(v0, v12)
;; @0056                               jump block7
;;
;;                                 block6:
;; @0056                               v55 = load.i64 notrap aligned readonly v0+40
;; @0056                               v57 = load.i64 notrap aligned readonly v0+48
;; @0056                               v58 = uextend.i64 v12
;; @0056                               v59 = iconst.i64 8
;; @0056                               v60 = uadd_overflow_trap v58, v59, user1  ; v59 = 8
;; @0056                               v61 = iconst.i64 8
;; @0056                               v62 = uadd_overflow_trap v60, v61, user1  ; v61 = 8
;; @0056                               v63 = icmp ule v62, v57
;; @0056                               trapz v63, user1
;; @0056                               v64 = iadd v55, v60
;; @0056                               store.i64 notrap aligned v51, v64
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly v0+88
;;                                     v66 = iconst.i64 2
;; @005f                               v8 = ishl v6, v66  ; v66 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i32 user5 aligned table v11
;;                                     v67 = iconst.i32 0
;; @005f                               v13 = icmp eq v3, v67  ; v67 = 0
;; @005f                               brif v13, block3, block2
;;
;;                                 block2:
;; @005f                               v15 = load.i64 notrap aligned readonly v0+40
;; @005f                               v17 = load.i64 notrap aligned readonly v0+48
;; @005f                               v18 = uextend.i64 v3
;; @005f                               v19 = iconst.i64 8
;; @005f                               v20 = uadd_overflow_trap v18, v19, user1  ; v19 = 8
;; @005f                               v21 = iconst.i64 8
;; @005f                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 8
;; @005f                               v23 = icmp ule v22, v17
;; @005f                               trapz v23, user1
;; @005f                               v24 = iadd v15, v20
;; @005f                               v25 = load.i64 notrap aligned v24
;;                                     v68 = iconst.i64 1
;; @005f                               v26 = iadd v25, v68  ; v68 = 1
;; @005f                               v28 = load.i64 notrap aligned readonly v0+40
;; @005f                               v30 = load.i64 notrap aligned readonly v0+48
;; @005f                               v31 = uextend.i64 v3
;; @005f                               v32 = iconst.i64 8
;; @005f                               v33 = uadd_overflow_trap v31, v32, user1  ; v32 = 8
;; @005f                               v34 = iconst.i64 8
;; @005f                               v35 = uadd_overflow_trap v33, v34, user1  ; v34 = 8
;; @005f                               v36 = icmp ule v35, v30
;; @005f                               trapz v36, user1
;; @005f                               v37 = iadd v28, v33
;; @005f                               store notrap aligned v26, v37
;; @005f                               jump block3
;;
;;                                 block3:
;; @005f                               store.i32 user5 aligned table v3, v11
;;                                     v69 = iconst.i32 0
;; @005f                               v38 = icmp.i32 eq v12, v69  ; v69 = 0
;; @005f                               brif v38, block7, block4
;;
;;                                 block4:
;; @005f                               v40 = load.i64 notrap aligned readonly v0+40
;; @005f                               v42 = load.i64 notrap aligned readonly v0+48
;; @005f                               v43 = uextend.i64 v12
;; @005f                               v44 = iconst.i64 8
;; @005f                               v45 = uadd_overflow_trap v43, v44, user1  ; v44 = 8
;; @005f                               v46 = iconst.i64 8
;; @005f                               v47 = uadd_overflow_trap v45, v46, user1  ; v46 = 8
;; @005f                               v48 = icmp ule v47, v42
;; @005f                               trapz v48, user1
;; @005f                               v49 = iadd v40, v45
;; @005f                               v50 = load.i64 notrap aligned v49
;;                                     v70 = iconst.i64 -1
;; @005f                               v51 = iadd v50, v70  ; v70 = -1
;;                                     v71 = iconst.i64 0
;; @005f                               v52 = icmp eq v51, v71  ; v71 = 0
;; @005f                               brif v52, block5, block6
;;
;;                                 block5 cold:
;; @005f                               call fn0(v0, v12)
;; @005f                               jump block7
;;
;;                                 block6:
;; @005f                               v55 = load.i64 notrap aligned readonly v0+40
;; @005f                               v57 = load.i64 notrap aligned readonly v0+48
;; @005f                               v58 = uextend.i64 v12
;; @005f                               v59 = iconst.i64 8
;; @005f                               v60 = uadd_overflow_trap v58, v59, user1  ; v59 = 8
;; @005f                               v61 = iconst.i64 8
;; @005f                               v62 = uadd_overflow_trap v60, v61, user1  ; v61 = 8
;; @005f                               v63 = icmp ule v62, v57
;; @005f                               trapz v63, user1
;; @005f                               v64 = iadd v55, v60
;; @005f                               store.i64 notrap aligned v51, v64
;; @005f                               jump block7
;;
;;                                 block7:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
