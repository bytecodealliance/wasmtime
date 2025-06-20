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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v62 = iconst.i64 2
;; @0056                               v8 = ishl v6, v62  ; v62 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               v12 = load.i32 user5 aligned table v11
;;                                     v61 = iconst.i32 1
;; @0056                               v13 = band v2, v61  ; v61 = 1
;;                                     v60 = iconst.i32 0
;; @0056                               v14 = icmp eq v2, v60  ; v60 = 0
;; @0056                               v15 = uextend.i32 v14
;; @0056                               v16 = bor v13, v15
;; @0056                               brif v16, block3, block2
;;
;;                                 block2:
;; @0056                               v17 = uextend.i64 v2
;; @0056                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @0056                               v18 = load.i64 notrap aligned readonly can_move v58+24
;; @0056                               v19 = iadd v18, v17
;; @0056                               v20 = iconst.i64 8
;; @0056                               v21 = iadd v19, v20  ; v20 = 8
;; @0056                               v22 = load.i64 notrap aligned v21
;;                                     v57 = iconst.i64 1
;; @0056                               v23 = iadd v22, v57  ; v57 = 1
;; @0056                               v24 = uextend.i64 v2
;; @0056                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @0056                               v25 = load.i64 notrap aligned readonly can_move v55+24
;; @0056                               v26 = iadd v25, v24
;; @0056                               v27 = iconst.i64 8
;; @0056                               v28 = iadd v26, v27  ; v27 = 8
;; @0056                               store notrap aligned v23, v28
;; @0056                               jump block3
;;
;;                                 block3:
;; @0056                               store.i32 user5 aligned table v2, v11
;;                                     v54 = iconst.i32 1
;; @0056                               v29 = band.i32 v12, v54  ; v54 = 1
;;                                     v53 = iconst.i32 0
;; @0056                               v30 = icmp.i32 eq v12, v53  ; v53 = 0
;; @0056                               v31 = uextend.i32 v30
;; @0056                               v32 = bor v29, v31
;; @0056                               brif v32, block7, block4
;;
;;                                 block4:
;; @0056                               v33 = uextend.i64 v12
;; @0056                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @0056                               v34 = load.i64 notrap aligned readonly can_move v51+24
;; @0056                               v35 = iadd v34, v33
;; @0056                               v36 = iconst.i64 8
;; @0056                               v37 = iadd v35, v36  ; v36 = 8
;; @0056                               v38 = load.i64 notrap aligned v37
;;                                     v50 = iconst.i64 -1
;; @0056                               v39 = iadd v38, v50  ; v50 = -1
;;                                     v49 = iconst.i64 0
;; @0056                               v40 = icmp eq v39, v49  ; v49 = 0
;; @0056                               brif v40, block5, block6
;;
;;                                 block5 cold:
;; @0056                               call fn0(v0, v12)
;; @0056                               jump block7
;;
;;                                 block6:
;; @0056                               v42 = uextend.i64 v12
;; @0056                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @0056                               v43 = load.i64 notrap aligned readonly can_move v47+24
;; @0056                               v44 = iadd v43, v42
;; @0056                               v45 = iconst.i64 8
;; @0056                               v46 = iadd v44, v45  ; v45 = 8
;; @0056                               store.i64 notrap aligned v39, v46
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv6 = load.i64 notrap aligned readonly can_move gv5+24
;;     gv7 = load.i64 notrap aligned gv5+32
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v62 = iconst.i64 2
;; @005f                               v8 = ishl v6, v62  ; v62 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               v12 = load.i32 user5 aligned table v11
;;                                     v61 = iconst.i32 1
;; @005f                               v13 = band v3, v61  ; v61 = 1
;;                                     v60 = iconst.i32 0
;; @005f                               v14 = icmp eq v3, v60  ; v60 = 0
;; @005f                               v15 = uextend.i32 v14
;; @005f                               v16 = bor v13, v15
;; @005f                               brif v16, block3, block2
;;
;;                                 block2:
;; @005f                               v17 = uextend.i64 v3
;; @005f                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @005f                               v18 = load.i64 notrap aligned readonly can_move v58+24
;; @005f                               v19 = iadd v18, v17
;; @005f                               v20 = iconst.i64 8
;; @005f                               v21 = iadd v19, v20  ; v20 = 8
;; @005f                               v22 = load.i64 notrap aligned v21
;;                                     v57 = iconst.i64 1
;; @005f                               v23 = iadd v22, v57  ; v57 = 1
;; @005f                               v24 = uextend.i64 v3
;; @005f                               v55 = load.i64 notrap aligned readonly can_move v0+8
;; @005f                               v25 = load.i64 notrap aligned readonly can_move v55+24
;; @005f                               v26 = iadd v25, v24
;; @005f                               v27 = iconst.i64 8
;; @005f                               v28 = iadd v26, v27  ; v27 = 8
;; @005f                               store notrap aligned v23, v28
;; @005f                               jump block3
;;
;;                                 block3:
;; @005f                               store.i32 user5 aligned table v3, v11
;;                                     v54 = iconst.i32 1
;; @005f                               v29 = band.i32 v12, v54  ; v54 = 1
;;                                     v53 = iconst.i32 0
;; @005f                               v30 = icmp.i32 eq v12, v53  ; v53 = 0
;; @005f                               v31 = uextend.i32 v30
;; @005f                               v32 = bor v29, v31
;; @005f                               brif v32, block7, block4
;;
;;                                 block4:
;; @005f                               v33 = uextend.i64 v12
;; @005f                               v51 = load.i64 notrap aligned readonly can_move v0+8
;; @005f                               v34 = load.i64 notrap aligned readonly can_move v51+24
;; @005f                               v35 = iadd v34, v33
;; @005f                               v36 = iconst.i64 8
;; @005f                               v37 = iadd v35, v36  ; v36 = 8
;; @005f                               v38 = load.i64 notrap aligned v37
;;                                     v50 = iconst.i64 -1
;; @005f                               v39 = iadd v38, v50  ; v50 = -1
;;                                     v49 = iconst.i64 0
;; @005f                               v40 = icmp eq v39, v49  ; v49 = 0
;; @005f                               brif v40, block5, block6
;;
;;                                 block5 cold:
;; @005f                               call fn0(v0, v12)
;; @005f                               jump block7
;;
;;                                 block6:
;; @005f                               v42 = uextend.i64 v12
;; @005f                               v47 = load.i64 notrap aligned readonly can_move v0+8
;; @005f                               v43 = load.i64 notrap aligned readonly can_move v47+24
;; @005f                               v44 = iadd v43, v42
;; @005f                               v45 = iconst.i64 8
;; @005f                               v46 = iadd v44, v45  ; v45 = 8
;; @005f                               store.i64 notrap aligned v39, v46
;; @005f                               jump block7
;;
;;                                 block7:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
