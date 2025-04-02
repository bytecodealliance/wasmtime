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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i64 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i64 notrap aligned v0+80
;; @0055                               v5 = ireduce.i32 v4
;; @0055                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0055                               v7 = uextend.i64 v3  ; v3 = 0
;; @0055                               v8 = load.i64 notrap aligned v0+72
;;                                     v74 = iconst.i64 2
;; @0055                               v9 = ishl v7, v74  ; v74 = 2
;; @0055                               v10 = iadd v8, v9
;; @0055                               v11 = iconst.i64 0
;; @0055                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0055                               v13 = load.i32 user5 aligned table v12
;;                                     v75 = iconst.i32 1
;; @0055                               v14 = band v2, v75  ; v75 = 1
;;                                     v76 = iconst.i32 0
;; @0055                               v15 = icmp eq v2, v76  ; v76 = 0
;; @0055                               v16 = uextend.i32 v15
;; @0055                               v17 = bor v14, v16
;; @0055                               brif v17, block3, block2
;;
;;                                 block2:
;; @0055                               v19 = load.i64 notrap aligned readonly can_move v0+40
;; @0055                               v21 = load.i64 notrap aligned readonly can_move v0+48
;; @0055                               v22 = uextend.i64 v2
;; @0055                               v23 = iconst.i64 8
;; @0055                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @0055                               v25 = iconst.i64 8
;; @0055                               v26 = uadd_overflow_trap v24, v25, user1  ; v25 = 8
;; @0055                               v27 = icmp ule v26, v21
;; @0055                               trapz v27, user1
;; @0055                               v28 = iadd v19, v24
;; @0055                               v29 = load.i64 notrap aligned v28
;;                                     v77 = iconst.i64 1
;; @0055                               v30 = iadd v29, v77  ; v77 = 1
;; @0055                               v32 = load.i64 notrap aligned readonly can_move v0+40
;; @0055                               v34 = load.i64 notrap aligned readonly can_move v0+48
;; @0055                               v35 = uextend.i64 v2
;; @0055                               v36 = iconst.i64 8
;; @0055                               v37 = uadd_overflow_trap v35, v36, user1  ; v36 = 8
;; @0055                               v38 = iconst.i64 8
;; @0055                               v39 = uadd_overflow_trap v37, v38, user1  ; v38 = 8
;; @0055                               v40 = icmp ule v39, v34
;; @0055                               trapz v40, user1
;; @0055                               v41 = iadd v32, v37
;; @0055                               store notrap aligned v30, v41
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               store.i32 user5 aligned table v2, v12
;;                                     v78 = iconst.i32 1
;; @0055                               v42 = band.i32 v13, v78  ; v78 = 1
;;                                     v79 = iconst.i32 0
;; @0055                               v43 = icmp.i32 eq v13, v79  ; v79 = 0
;; @0055                               v44 = uextend.i32 v43
;; @0055                               v45 = bor v42, v44
;; @0055                               brif v45, block7, block4
;;
;;                                 block4:
;; @0055                               v47 = load.i64 notrap aligned readonly can_move v0+40
;; @0055                               v49 = load.i64 notrap aligned readonly can_move v0+48
;; @0055                               v50 = uextend.i64 v13
;; @0055                               v51 = iconst.i64 8
;; @0055                               v52 = uadd_overflow_trap v50, v51, user1  ; v51 = 8
;; @0055                               v53 = iconst.i64 8
;; @0055                               v54 = uadd_overflow_trap v52, v53, user1  ; v53 = 8
;; @0055                               v55 = icmp ule v54, v49
;; @0055                               trapz v55, user1
;; @0055                               v56 = iadd v47, v52
;; @0055                               v57 = load.i64 notrap aligned v56
;;                                     v80 = iconst.i64 -1
;; @0055                               v58 = iadd v57, v80  ; v80 = -1
;;                                     v81 = iconst.i64 0
;; @0055                               v59 = icmp eq v58, v81  ; v81 = 0
;; @0055                               brif v59, block5, block6
;;
;;                                 block5 cold:
;; @0055                               call fn0(v0, v13)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v62 = load.i64 notrap aligned readonly can_move v0+40
;; @0055                               v64 = load.i64 notrap aligned readonly can_move v0+48
;; @0055                               v65 = uextend.i64 v13
;; @0055                               v66 = iconst.i64 8
;; @0055                               v67 = uadd_overflow_trap v65, v66, user1  ; v66 = 8
;; @0055                               v68 = iconst.i64 8
;; @0055                               v69 = uadd_overflow_trap v67, v68, user1  ; v68 = 8
;; @0055                               v70 = icmp ule v69, v64
;; @0055                               trapz v70, user1
;; @0055                               v71 = iadd v62, v67
;; @0055                               store.i64 notrap aligned v58, v71
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i64 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i64 notrap aligned v0+80
;; @005e                               v5 = ireduce.i32 v4
;; @005e                               v6 = icmp uge v2, v5
;; @005e                               v7 = uextend.i64 v2
;; @005e                               v8 = load.i64 notrap aligned v0+72
;;                                     v74 = iconst.i64 2
;; @005e                               v9 = ishl v7, v74  ; v74 = 2
;; @005e                               v10 = iadd v8, v9
;; @005e                               v11 = iconst.i64 0
;; @005e                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005e                               v13 = load.i32 user5 aligned table v12
;;                                     v75 = iconst.i32 1
;; @005e                               v14 = band v3, v75  ; v75 = 1
;;                                     v76 = iconst.i32 0
;; @005e                               v15 = icmp eq v3, v76  ; v76 = 0
;; @005e                               v16 = uextend.i32 v15
;; @005e                               v17 = bor v14, v16
;; @005e                               brif v17, block3, block2
;;
;;                                 block2:
;; @005e                               v19 = load.i64 notrap aligned readonly can_move v0+40
;; @005e                               v21 = load.i64 notrap aligned readonly can_move v0+48
;; @005e                               v22 = uextend.i64 v3
;; @005e                               v23 = iconst.i64 8
;; @005e                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @005e                               v25 = iconst.i64 8
;; @005e                               v26 = uadd_overflow_trap v24, v25, user1  ; v25 = 8
;; @005e                               v27 = icmp ule v26, v21
;; @005e                               trapz v27, user1
;; @005e                               v28 = iadd v19, v24
;; @005e                               v29 = load.i64 notrap aligned v28
;;                                     v77 = iconst.i64 1
;; @005e                               v30 = iadd v29, v77  ; v77 = 1
;; @005e                               v32 = load.i64 notrap aligned readonly can_move v0+40
;; @005e                               v34 = load.i64 notrap aligned readonly can_move v0+48
;; @005e                               v35 = uextend.i64 v3
;; @005e                               v36 = iconst.i64 8
;; @005e                               v37 = uadd_overflow_trap v35, v36, user1  ; v36 = 8
;; @005e                               v38 = iconst.i64 8
;; @005e                               v39 = uadd_overflow_trap v37, v38, user1  ; v38 = 8
;; @005e                               v40 = icmp ule v39, v34
;; @005e                               trapz v40, user1
;; @005e                               v41 = iadd v32, v37
;; @005e                               store notrap aligned v30, v41
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               store.i32 user5 aligned table v3, v12
;;                                     v78 = iconst.i32 1
;; @005e                               v42 = band.i32 v13, v78  ; v78 = 1
;;                                     v79 = iconst.i32 0
;; @005e                               v43 = icmp.i32 eq v13, v79  ; v79 = 0
;; @005e                               v44 = uextend.i32 v43
;; @005e                               v45 = bor v42, v44
;; @005e                               brif v45, block7, block4
;;
;;                                 block4:
;; @005e                               v47 = load.i64 notrap aligned readonly can_move v0+40
;; @005e                               v49 = load.i64 notrap aligned readonly can_move v0+48
;; @005e                               v50 = uextend.i64 v13
;; @005e                               v51 = iconst.i64 8
;; @005e                               v52 = uadd_overflow_trap v50, v51, user1  ; v51 = 8
;; @005e                               v53 = iconst.i64 8
;; @005e                               v54 = uadd_overflow_trap v52, v53, user1  ; v53 = 8
;; @005e                               v55 = icmp ule v54, v49
;; @005e                               trapz v55, user1
;; @005e                               v56 = iadd v47, v52
;; @005e                               v57 = load.i64 notrap aligned v56
;;                                     v80 = iconst.i64 -1
;; @005e                               v58 = iadd v57, v80  ; v80 = -1
;;                                     v81 = iconst.i64 0
;; @005e                               v59 = icmp eq v58, v81  ; v81 = 0
;; @005e                               brif v59, block5, block6
;;
;;                                 block5 cold:
;; @005e                               call fn0(v0, v13)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v62 = load.i64 notrap aligned readonly can_move v0+40
;; @005e                               v64 = load.i64 notrap aligned readonly can_move v0+48
;; @005e                               v65 = uextend.i64 v13
;; @005e                               v66 = iconst.i64 8
;; @005e                               v67 = uadd_overflow_trap v65, v66, user1  ; v66 = 8
;; @005e                               v68 = iconst.i64 8
;; @005e                               v69 = uadd_overflow_trap v67, v68, user1  ; v68 = 8
;; @005e                               v70 = icmp ule v69, v64
;; @005e                               trapz v70, user1
;; @005e                               v71 = iadd v62, v67
;; @005e                               store.i64 notrap aligned v58, v71
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
