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

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i64 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+80
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+72
;;                                     v59 = iconst.i64 2
;; @0053                               v9 = ishl v7, v59  ; v59 = 2
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 user5 aligned table v12
;;                                     v60 = stack_addr.i64 ss0
;;                                     store notrap v13, v60
;;                                     v61 = stack_addr.i64 ss0
;;                                     v56 = load.i32 notrap v61
;;                                     v62 = iconst.i32 1
;; @0053                               v14 = band v56, v62  ; v62 = 1
;;                                     v63 = stack_addr.i64 ss0
;;                                     v55 = load.i32 notrap v63
;;                                     v64 = iconst.i32 0
;; @0053                               v15 = icmp eq v55, v64  ; v64 = 0
;; @0053                               v16 = uextend.i32 v15
;; @0053                               v17 = bor v14, v16
;; @0053                               brif v17, block5, block2
;;
;;                                 block2:
;; @0053                               v19 = load.i64 notrap aligned readonly v0+56
;; @0053                               v20 = load.i64 notrap aligned v19
;; @0053                               v21 = load.i64 notrap aligned v19+8
;; @0053                               v22 = icmp eq v20, v21
;; @0053                               brif v22, block3, block4
;;
;;                                 block4:
;; @0053                               v24 = load.i64 notrap aligned readonly can_move v0+40
;; @0053                               v26 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v65 = stack_addr.i64 ss0
;;                                     v54 = load.i32 notrap v65
;; @0053                               v27 = uextend.i64 v54
;; @0053                               v28 = iconst.i64 8
;; @0053                               v29 = uadd_overflow_trap v27, v28, user1  ; v28 = 8
;; @0053                               v30 = iconst.i64 8
;; @0053                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @0053                               v32 = icmp ule v31, v26
;; @0053                               trapz v32, user1
;; @0053                               v33 = iadd v24, v29
;; @0053                               v34 = load.i64 notrap aligned v33
;;                                     v66 = iconst.i64 1
;; @0053                               v35 = iadd v34, v66  ; v66 = 1
;; @0053                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0053                               v39 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v67 = stack_addr.i64 ss0
;;                                     v53 = load.i32 notrap v67
;; @0053                               v40 = uextend.i64 v53
;; @0053                               v41 = iconst.i64 8
;; @0053                               v42 = uadd_overflow_trap v40, v41, user1  ; v41 = 8
;; @0053                               v43 = iconst.i64 8
;; @0053                               v44 = uadd_overflow_trap v42, v43, user1  ; v43 = 8
;; @0053                               v45 = icmp ule v44, v39
;; @0053                               trapz v45, user1
;; @0053                               v46 = iadd v37, v42
;; @0053                               store notrap aligned v35, v46
;;                                     v68 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v68
;; @0053                               store notrap aligned v52, v20
;;                                     v69 = iconst.i64 4
;; @0053                               v47 = iadd.i64 v20, v69  ; v69 = 4
;; @0053                               store notrap aligned v47, v19
;; @0053                               jump block5
;;
;;                                 block3 cold:
;;                                     v70 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v70
;; @0053                               v49 = call fn0(v0, v51), stack_map=[i32 @ ss0+0]
;; @0053                               jump block5
;;
;;                                 block5:
;;                                     v71 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v71
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v50
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i64 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+80
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+72
;;                                     v59 = iconst.i64 2
;; @005a                               v9 = ishl v7, v59  ; v59 = 2
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 user5 aligned table v12
;;                                     v60 = stack_addr.i64 ss0
;;                                     store notrap v13, v60
;;                                     v61 = stack_addr.i64 ss0
;;                                     v56 = load.i32 notrap v61
;;                                     v62 = iconst.i32 1
;; @005a                               v14 = band v56, v62  ; v62 = 1
;;                                     v63 = stack_addr.i64 ss0
;;                                     v55 = load.i32 notrap v63
;;                                     v64 = iconst.i32 0
;; @005a                               v15 = icmp eq v55, v64  ; v64 = 0
;; @005a                               v16 = uextend.i32 v15
;; @005a                               v17 = bor v14, v16
;; @005a                               brif v17, block5, block2
;;
;;                                 block2:
;; @005a                               v19 = load.i64 notrap aligned readonly v0+56
;; @005a                               v20 = load.i64 notrap aligned v19
;; @005a                               v21 = load.i64 notrap aligned v19+8
;; @005a                               v22 = icmp eq v20, v21
;; @005a                               brif v22, block3, block4
;;
;;                                 block4:
;; @005a                               v24 = load.i64 notrap aligned readonly can_move v0+40
;; @005a                               v26 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v65 = stack_addr.i64 ss0
;;                                     v54 = load.i32 notrap v65
;; @005a                               v27 = uextend.i64 v54
;; @005a                               v28 = iconst.i64 8
;; @005a                               v29 = uadd_overflow_trap v27, v28, user1  ; v28 = 8
;; @005a                               v30 = iconst.i64 8
;; @005a                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @005a                               v32 = icmp ule v31, v26
;; @005a                               trapz v32, user1
;; @005a                               v33 = iadd v24, v29
;; @005a                               v34 = load.i64 notrap aligned v33
;;                                     v66 = iconst.i64 1
;; @005a                               v35 = iadd v34, v66  ; v66 = 1
;; @005a                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @005a                               v39 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v67 = stack_addr.i64 ss0
;;                                     v53 = load.i32 notrap v67
;; @005a                               v40 = uextend.i64 v53
;; @005a                               v41 = iconst.i64 8
;; @005a                               v42 = uadd_overflow_trap v40, v41, user1  ; v41 = 8
;; @005a                               v43 = iconst.i64 8
;; @005a                               v44 = uadd_overflow_trap v42, v43, user1  ; v43 = 8
;; @005a                               v45 = icmp ule v44, v39
;; @005a                               trapz v45, user1
;; @005a                               v46 = iadd v37, v42
;; @005a                               store notrap aligned v35, v46
;;                                     v68 = stack_addr.i64 ss0
;;                                     v52 = load.i32 notrap v68
;; @005a                               store notrap aligned v52, v20
;;                                     v69 = iconst.i64 4
;; @005a                               v47 = iadd.i64 v20, v69  ; v69 = 4
;; @005a                               store notrap aligned v47, v19
;; @005a                               jump block5
;;
;;                                 block3 cold:
;;                                     v70 = stack_addr.i64 ss0
;;                                     v51 = load.i32 notrap v70
;; @005a                               v49 = call fn0(v0, v51), stack_map=[i32 @ ss0+0]
;; @005a                               jump block5
;;
;;                                 block5:
;;                                     v71 = stack_addr.i64 ss0
;;                                     v50 = load.i32 notrap v71
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v50
;; }
