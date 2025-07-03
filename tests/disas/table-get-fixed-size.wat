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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v61 = iconst.i64 2
;; @0054                               v8 = ishl v6, v61  ; v61 = 2
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = iconst.i64 0
;; @0054                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0054                               v12 = load.i32 user5 aligned table v11
;;                                     v60 = iconst.i32 1
;; @0054                               v13 = band v12, v60  ; v60 = 1
;;                                     v59 = iconst.i32 0
;; @0054                               v14 = icmp eq v12, v59  ; v59 = 0
;; @0054                               v15 = uextend.i32 v14
;; @0054                               v16 = bor v13, v15
;; @0054                               brif v16, block4, block2
;;
;;                                 block2:
;; @0054                               v17 = uextend.i64 v12
;; @0054                               v57 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v18 = load.i64 notrap aligned readonly can_move v57+24
;; @0054                               v19 = iadd v18, v17
;; @0054                               v20 = load.i32 notrap aligned v19
;; @0054                               v21 = iconst.i32 2
;; @0054                               v22 = band v20, v21  ; v21 = 2
;; @0054                               brif v22, block4, block3
;;
;;                                 block3:
;; @0054                               v24 = load.i64 notrap aligned readonly v0+32
;; @0054                               v25 = load.i32 notrap aligned v24
;; @0054                               v26 = uextend.i64 v12
;; @0054                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v27 = load.i64 notrap aligned readonly can_move v55+24
;; @0054                               v28 = iadd v27, v26
;; @0054                               v29 = iconst.i64 16
;; @0054                               v30 = iadd v28, v29  ; v29 = 16
;; @0054                               store notrap aligned v25, v30
;; @0054                               v31 = iconst.i32 2
;; @0054                               v32 = bor.i32 v20, v31  ; v31 = 2
;; @0054                               v33 = uextend.i64 v12
;; @0054                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v34 = load.i64 notrap aligned readonly can_move v53+24
;; @0054                               v35 = iadd v34, v33
;; @0054                               store notrap aligned v32, v35
;; @0054                               v36 = uextend.i64 v12
;; @0054                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v37 = load.i64 notrap aligned readonly can_move v51+24
;; @0054                               v38 = iadd v37, v36
;; @0054                               v39 = iconst.i64 8
;; @0054                               v40 = iadd v38, v39  ; v39 = 8
;; @0054                               v41 = load.i64 notrap aligned v40
;;                                     v50 = iconst.i64 1
;; @0054                               v42 = iadd v41, v50  ; v50 = 1
;; @0054                               v43 = uextend.i64 v12
;; @0054                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @0054                               v44 = load.i64 notrap aligned readonly can_move v48+24
;; @0054                               v45 = iadd v44, v43
;; @0054                               v46 = iconst.i64 8
;; @0054                               v47 = iadd v45, v46  ; v46 = 8
;; @0054                               store notrap aligned v42, v47
;; @0054                               store.i32 notrap aligned v12, v24
;; @0054                               jump block4
;;
;;                                 block4:
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v12
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v61 = iconst.i64 2
;; @005b                               v8 = ishl v6, v61  ; v61 = 2
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = iconst.i64 0
;; @005b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005b                               v12 = load.i32 user5 aligned table v11
;;                                     v60 = iconst.i32 1
;; @005b                               v13 = band v12, v60  ; v60 = 1
;;                                     v59 = iconst.i32 0
;; @005b                               v14 = icmp eq v12, v59  ; v59 = 0
;; @005b                               v15 = uextend.i32 v14
;; @005b                               v16 = bor v13, v15
;; @005b                               brif v16, block4, block2
;;
;;                                 block2:
;; @005b                               v17 = uextend.i64 v12
;; @005b                               v57 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v18 = load.i64 notrap aligned readonly can_move v57+24
;; @005b                               v19 = iadd v18, v17
;; @005b                               v20 = load.i32 notrap aligned v19
;; @005b                               v21 = iconst.i32 2
;; @005b                               v22 = band v20, v21  ; v21 = 2
;; @005b                               brif v22, block4, block3
;;
;;                                 block3:
;; @005b                               v24 = load.i64 notrap aligned readonly v0+32
;; @005b                               v25 = load.i32 notrap aligned v24
;; @005b                               v26 = uextend.i64 v12
;; @005b                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v27 = load.i64 notrap aligned readonly can_move v55+24
;; @005b                               v28 = iadd v27, v26
;; @005b                               v29 = iconst.i64 16
;; @005b                               v30 = iadd v28, v29  ; v29 = 16
;; @005b                               store notrap aligned v25, v30
;; @005b                               v31 = iconst.i32 2
;; @005b                               v32 = bor.i32 v20, v31  ; v31 = 2
;; @005b                               v33 = uextend.i64 v12
;; @005b                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v34 = load.i64 notrap aligned readonly can_move v53+24
;; @005b                               v35 = iadd v34, v33
;; @005b                               store notrap aligned v32, v35
;; @005b                               v36 = uextend.i64 v12
;; @005b                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v37 = load.i64 notrap aligned readonly can_move v51+24
;; @005b                               v38 = iadd v37, v36
;; @005b                               v39 = iconst.i64 8
;; @005b                               v40 = iadd v38, v39  ; v39 = 8
;; @005b                               v41 = load.i64 notrap aligned v40
;;                                     v50 = iconst.i64 1
;; @005b                               v42 = iadd v41, v50  ; v50 = 1
;; @005b                               v43 = uextend.i64 v12
;; @005b                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @005b                               v44 = load.i64 notrap aligned readonly can_move v48+24
;; @005b                               v45 = iadd v44, v43
;; @005b                               v46 = iconst.i64 8
;; @005b                               v47 = iadd v45, v46  ; v46 = 8
;; @005b                               store notrap aligned v42, v47
;; @005b                               store.i32 notrap aligned v12, v24
;; @005b                               jump block4
;;
;;                                 block4:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v12
;; }
