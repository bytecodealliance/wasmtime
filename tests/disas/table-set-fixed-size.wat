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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v72 = iconst.i64 2
;; @0056                               v8 = ishl v6, v72  ; v72 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               v12 = load.i32 user5 aligned table v11
;;                                     v73 = iconst.i32 1
;; @0056                               v13 = band v2, v73  ; v73 = 1
;;                                     v74 = iconst.i32 0
;; @0056                               v14 = icmp eq v2, v74  ; v74 = 0
;; @0056                               v15 = uextend.i32 v14
;; @0056                               v16 = bor v13, v15
;; @0056                               brif v16, block3, block2
;;
;;                                 block2:
;; @0056                               v18 = load.i64 notrap aligned readonly can_move v0+40
;; @0056                               v20 = load.i64 notrap aligned readonly can_move v0+48
;; @0056                               v21 = uextend.i64 v2
;; @0056                               v22 = iconst.i64 8
;; @0056                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @0056                               v24 = iconst.i64 8
;; @0056                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @0056                               v26 = icmp ule v25, v20
;; @0056                               trapz v26, user1
;; @0056                               v27 = iadd v18, v23
;; @0056                               v28 = load.i64 notrap aligned v27
;;                                     v75 = iconst.i64 1
;; @0056                               v29 = iadd v28, v75  ; v75 = 1
;; @0056                               v31 = load.i64 notrap aligned readonly can_move v0+40
;; @0056                               v33 = load.i64 notrap aligned readonly can_move v0+48
;; @0056                               v34 = uextend.i64 v2
;; @0056                               v35 = iconst.i64 8
;; @0056                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @0056                               v37 = iconst.i64 8
;; @0056                               v38 = uadd_overflow_trap v36, v37, user1  ; v37 = 8
;; @0056                               v39 = icmp ule v38, v33
;; @0056                               trapz v39, user1
;; @0056                               v40 = iadd v31, v36
;; @0056                               store notrap aligned v29, v40
;; @0056                               jump block3
;;
;;                                 block3:
;; @0056                               store.i32 user5 aligned table v2, v11
;;                                     v76 = iconst.i32 1
;; @0056                               v41 = band.i32 v12, v76  ; v76 = 1
;;                                     v77 = iconst.i32 0
;; @0056                               v42 = icmp.i32 eq v12, v77  ; v77 = 0
;; @0056                               v43 = uextend.i32 v42
;; @0056                               v44 = bor v41, v43
;; @0056                               brif v44, block7, block4
;;
;;                                 block4:
;; @0056                               v46 = load.i64 notrap aligned readonly can_move v0+40
;; @0056                               v48 = load.i64 notrap aligned readonly can_move v0+48
;; @0056                               v49 = uextend.i64 v12
;; @0056                               v50 = iconst.i64 8
;; @0056                               v51 = uadd_overflow_trap v49, v50, user1  ; v50 = 8
;; @0056                               v52 = iconst.i64 8
;; @0056                               v53 = uadd_overflow_trap v51, v52, user1  ; v52 = 8
;; @0056                               v54 = icmp ule v53, v48
;; @0056                               trapz v54, user1
;; @0056                               v55 = iadd v46, v51
;; @0056                               v56 = load.i64 notrap aligned v55
;;                                     v78 = iconst.i64 -1
;; @0056                               v57 = iadd v56, v78  ; v78 = -1
;;                                     v79 = iconst.i64 0
;; @0056                               v58 = icmp eq v57, v79  ; v79 = 0
;; @0056                               brif v58, block5, block6
;;
;;                                 block5 cold:
;; @0056                               call fn0(v0, v12)
;; @0056                               jump block7
;;
;;                                 block6:
;; @0056                               v61 = load.i64 notrap aligned readonly can_move v0+40
;; @0056                               v63 = load.i64 notrap aligned readonly can_move v0+48
;; @0056                               v64 = uextend.i64 v12
;; @0056                               v65 = iconst.i64 8
;; @0056                               v66 = uadd_overflow_trap v64, v65, user1  ; v65 = 8
;; @0056                               v67 = iconst.i64 8
;; @0056                               v68 = uadd_overflow_trap v66, v67, user1  ; v67 = 8
;; @0056                               v69 = icmp ule v68, v63
;; @0056                               trapz v69, user1
;; @0056                               v70 = iadd v61, v66
;; @0056                               store.i64 notrap aligned v57, v70
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v72 = iconst.i64 2
;; @005f                               v8 = ishl v6, v72  ; v72 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i32 user5 aligned table v11
;;                                     v73 = iconst.i32 1
;; @005f                               v13 = band v3, v73  ; v73 = 1
;;                                     v74 = iconst.i32 0
;; @005f                               v14 = icmp eq v3, v74  ; v74 = 0
;; @005f                               v15 = uextend.i32 v14
;; @005f                               v16 = bor v13, v15
;; @005f                               brif v16, block3, block2
;;
;;                                 block2:
;; @005f                               v18 = load.i64 notrap aligned readonly can_move v0+40
;; @005f                               v20 = load.i64 notrap aligned readonly can_move v0+48
;; @005f                               v21 = uextend.i64 v3
;; @005f                               v22 = iconst.i64 8
;; @005f                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 8
;; @005f                               v24 = iconst.i64 8
;; @005f                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @005f                               v26 = icmp ule v25, v20
;; @005f                               trapz v26, user1
;; @005f                               v27 = iadd v18, v23
;; @005f                               v28 = load.i64 notrap aligned v27
;;                                     v75 = iconst.i64 1
;; @005f                               v29 = iadd v28, v75  ; v75 = 1
;; @005f                               v31 = load.i64 notrap aligned readonly can_move v0+40
;; @005f                               v33 = load.i64 notrap aligned readonly can_move v0+48
;; @005f                               v34 = uextend.i64 v3
;; @005f                               v35 = iconst.i64 8
;; @005f                               v36 = uadd_overflow_trap v34, v35, user1  ; v35 = 8
;; @005f                               v37 = iconst.i64 8
;; @005f                               v38 = uadd_overflow_trap v36, v37, user1  ; v37 = 8
;; @005f                               v39 = icmp ule v38, v33
;; @005f                               trapz v39, user1
;; @005f                               v40 = iadd v31, v36
;; @005f                               store notrap aligned v29, v40
;; @005f                               jump block3
;;
;;                                 block3:
;; @005f                               store.i32 user5 aligned table v3, v11
;;                                     v76 = iconst.i32 1
;; @005f                               v41 = band.i32 v12, v76  ; v76 = 1
;;                                     v77 = iconst.i32 0
;; @005f                               v42 = icmp.i32 eq v12, v77  ; v77 = 0
;; @005f                               v43 = uextend.i32 v42
;; @005f                               v44 = bor v41, v43
;; @005f                               brif v44, block7, block4
;;
;;                                 block4:
;; @005f                               v46 = load.i64 notrap aligned readonly can_move v0+40
;; @005f                               v48 = load.i64 notrap aligned readonly can_move v0+48
;; @005f                               v49 = uextend.i64 v12
;; @005f                               v50 = iconst.i64 8
;; @005f                               v51 = uadd_overflow_trap v49, v50, user1  ; v50 = 8
;; @005f                               v52 = iconst.i64 8
;; @005f                               v53 = uadd_overflow_trap v51, v52, user1  ; v52 = 8
;; @005f                               v54 = icmp ule v53, v48
;; @005f                               trapz v54, user1
;; @005f                               v55 = iadd v46, v51
;; @005f                               v56 = load.i64 notrap aligned v55
;;                                     v78 = iconst.i64 -1
;; @005f                               v57 = iadd v56, v78  ; v78 = -1
;;                                     v79 = iconst.i64 0
;; @005f                               v58 = icmp eq v57, v79  ; v79 = 0
;; @005f                               brif v58, block5, block6
;;
;;                                 block5 cold:
;; @005f                               call fn0(v0, v12)
;; @005f                               jump block7
;;
;;                                 block6:
;; @005f                               v61 = load.i64 notrap aligned readonly can_move v0+40
;; @005f                               v63 = load.i64 notrap aligned readonly can_move v0+48
;; @005f                               v64 = uextend.i64 v12
;; @005f                               v65 = iconst.i64 8
;; @005f                               v66 = uadd_overflow_trap v64, v65, user1  ; v65 = 8
;; @005f                               v67 = iconst.i64 8
;; @005f                               v68 = uadd_overflow_trap v66, v67, user1  ; v67 = 8
;; @005f                               v69 = icmp ule v68, v63
;; @005f                               trapz v69, user1
;; @005f                               v70 = iadd v61, v66
;; @005f                               store.i64 notrap aligned v57, v70
;; @005f                               jump block7
;;
;;                                 block7:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
