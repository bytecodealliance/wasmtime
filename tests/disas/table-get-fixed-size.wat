;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-O opt-level=0"

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimized but with `opt-level=0` to legalize away
;; macro instructions.

(module
  (table (export "table") 7 7 externref)
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+56
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly can_move v0+56
;;                                     v45 = iconst.i64 2
;; @0054                               v8 = ishl v6, v45  ; v45 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user5 aligned table v11
;;                                     v46 = stack_addr.i64 ss0
;;                                     store notrap v12, v46
;;                                     v47 = stack_addr.i64 ss0
;;                                     v43 = load.i32 notrap v47
;;                                     v48 = iconst.i32 1
;; @0054                               v13 = band v43, v48  ; v48 = 1
;;                                     v49 = stack_addr.i64 ss0
;;                                     v42 = load.i32 notrap v49
;;                                     v50 = iconst.i32 0
;; @0054                               v14 = icmp eq v42, v50  ; v50 = 0
;; @0054                               v15 = uextend.i32 v14
;; @0054                               v16 = bor v13, v15
;; @0054                               brif v16, block5, block2
;;
;;                                 block2:
;; @0054                               v18 = load.i64 notrap aligned readonly v0+40
;; @0054                               v19 = load.i64 notrap aligned v18
;; @0054                               v20 = load.i64 notrap aligned v18+8
;; @0054                               v21 = icmp eq v19, v20
;; @0054                               brif v21, block3, block4
;;
;;                                 block4:
;;                                     v51 = stack_addr.i64 ss0
;;                                     v41 = load.i32 notrap v51
;; @0054                               v22 = uextend.i64 v41
;; @0054                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v23 = load.i64 notrap aligned readonly can_move v52+24
;; @0054                               v24 = iadd v23, v22
;; @0054                               v25 = iconst.i64 8
;; @0054                               v26 = iadd v24, v25  ; v25 = 8
;; @0054                               v27 = load.i64 notrap aligned v26
;;                                     v54 = iconst.i64 1
;; @0054                               v28 = iadd v27, v54  ; v54 = 1
;;                                     v55 = stack_addr.i64 ss0
;;                                     v40 = load.i32 notrap v55
;; @0054                               v29 = uextend.i64 v40
;; @0054                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v30 = load.i64 notrap aligned readonly can_move v56+24
;; @0054                               v31 = iadd v30, v29
;; @0054                               v32 = iconst.i64 8
;; @0054                               v33 = iadd v31, v32  ; v32 = 8
;; @0054                               store notrap aligned v28, v33
;;                                     v58 = stack_addr.i64 ss0
;;                                     v39 = load.i32 notrap v58
;; @0054                               store notrap aligned v39, v19
;;                                     v59 = iconst.i64 4
;; @0054                               v34 = iadd.i64 v19, v59  ; v59 = 4
;; @0054                               store notrap aligned v34, v18
;; @0054                               jump block5
;;
;;                                 block3 cold:
;;                                     v60 = stack_addr.i64 ss0
;;                                     v38 = load.i32 notrap v60
;; @0054                               v36 = call fn0(v0, v38), stack_map=[i32 @ ss0+0]
;; @0054                               jump block5
;;
;;                                 block5:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v37 = load.i32 notrap v61
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v37
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+56
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v0+56
;;                                     v45 = iconst.i64 2
;; @005b                               v8 = ishl v6, v45  ; v45 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user5 aligned table v11
;;                                     v46 = stack_addr.i64 ss0
;;                                     store notrap v12, v46
;;                                     v47 = stack_addr.i64 ss0
;;                                     v43 = load.i32 notrap v47
;;                                     v48 = iconst.i32 1
;; @005b                               v13 = band v43, v48  ; v48 = 1
;;                                     v49 = stack_addr.i64 ss0
;;                                     v42 = load.i32 notrap v49
;;                                     v50 = iconst.i32 0
;; @005b                               v14 = icmp eq v42, v50  ; v50 = 0
;; @005b                               v15 = uextend.i32 v14
;; @005b                               v16 = bor v13, v15
;; @005b                               brif v16, block5, block2
;;
;;                                 block2:
;; @005b                               v18 = load.i64 notrap aligned readonly v0+40
;; @005b                               v19 = load.i64 notrap aligned v18
;; @005b                               v20 = load.i64 notrap aligned v18+8
;; @005b                               v21 = icmp eq v19, v20
;; @005b                               brif v21, block3, block4
;;
;;                                 block4:
;;                                     v51 = stack_addr.i64 ss0
;;                                     v41 = load.i32 notrap v51
;; @005b                               v22 = uextend.i64 v41
;; @005b                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v23 = load.i64 notrap aligned readonly can_move v52+24
;; @005b                               v24 = iadd v23, v22
;; @005b                               v25 = iconst.i64 8
;; @005b                               v26 = iadd v24, v25  ; v25 = 8
;; @005b                               v27 = load.i64 notrap aligned v26
;;                                     v54 = iconst.i64 1
;; @005b                               v28 = iadd v27, v54  ; v54 = 1
;;                                     v55 = stack_addr.i64 ss0
;;                                     v40 = load.i32 notrap v55
;; @005b                               v29 = uextend.i64 v40
;; @005b                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v30 = load.i64 notrap aligned readonly can_move v56+24
;; @005b                               v31 = iadd v30, v29
;; @005b                               v32 = iconst.i64 8
;; @005b                               v33 = iadd v31, v32  ; v32 = 8
;; @005b                               store notrap aligned v28, v33
;;                                     v58 = stack_addr.i64 ss0
;;                                     v39 = load.i32 notrap v58
;; @005b                               store notrap aligned v39, v19
;;                                     v59 = iconst.i64 4
;; @005b                               v34 = iadd.i64 v19, v59  ; v59 = 4
;; @005b                               store notrap aligned v34, v18
;; @005b                               jump block5
;;
;;                                 block3 cold:
;;                                     v60 = stack_addr.i64 ss0
;;                                     v38 = load.i32 notrap v60
;; @005b                               v36 = call fn0(v0, v38), stack_map=[i32 @ ss0+0]
;; @005b                               jump block5
;;
;;                                 block5:
;;                                     v61 = stack_addr.i64 ss0
;;                                     v37 = load.i32 notrap v61
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v37
;; }
