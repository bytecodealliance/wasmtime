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
;;     gv4 = load.i64 notrap aligned gv3+56
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i64 notrap aligned v0+64
;; @0055                               v5 = ireduce.i32 v4
;; @0055                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0055                               v7 = uextend.i64 v3  ; v3 = 0
;; @0055                               v8 = load.i64 notrap aligned v0+56
;;                                     v63 = iconst.i64 2
;; @0055                               v9 = ishl v7, v63  ; v63 = 2
;; @0055                               v10 = iadd v8, v9
;; @0055                               v11 = iconst.i64 0
;; @0055                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0055                               v13 = load.i32 user5 aligned table v12
;;                                     v62 = iconst.i32 1
;; @0055                               v14 = band v2, v62  ; v62 = 1
;;                                     v61 = iconst.i32 0
;; @0055                               v15 = icmp eq v2, v61  ; v61 = 0
;; @0055                               v16 = uextend.i32 v15
;; @0055                               v17 = bor v14, v16
;; @0055                               brif v17, block3, block2
;;
;;                                 block2:
;; @0055                               v18 = uextend.i64 v2
;; @0055                               v59 = load.i64 notrap aligned readonly can_move v0+8
;; @0055                               v19 = load.i64 notrap aligned readonly can_move v59+24
;; @0055                               v20 = iadd v19, v18
;; @0055                               v21 = iconst.i64 8
;; @0055                               v22 = iadd v20, v21  ; v21 = 8
;; @0055                               v23 = load.i64 notrap aligned v22
;;                                     v58 = iconst.i64 1
;; @0055                               v24 = iadd v23, v58  ; v58 = 1
;; @0055                               v25 = uextend.i64 v2
;; @0055                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @0055                               v26 = load.i64 notrap aligned readonly can_move v56+24
;; @0055                               v27 = iadd v26, v25
;; @0055                               v28 = iconst.i64 8
;; @0055                               v29 = iadd v27, v28  ; v28 = 8
;; @0055                               store notrap aligned v24, v29
;; @0055                               jump block3
;;
;;                                 block3:
;; @0055                               store.i32 user5 aligned table v2, v12
;;                                     v55 = iconst.i32 1
;; @0055                               v30 = band.i32 v13, v55  ; v55 = 1
;;                                     v54 = iconst.i32 0
;; @0055                               v31 = icmp.i32 eq v13, v54  ; v54 = 0
;; @0055                               v32 = uextend.i32 v31
;; @0055                               v33 = bor v30, v32
;; @0055                               brif v33, block7, block4
;;
;;                                 block4:
;; @0055                               v34 = uextend.i64 v13
;; @0055                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @0055                               v35 = load.i64 notrap aligned readonly can_move v52+24
;; @0055                               v36 = iadd v35, v34
;; @0055                               v37 = iconst.i64 8
;; @0055                               v38 = iadd v36, v37  ; v37 = 8
;; @0055                               v39 = load.i64 notrap aligned v38
;;                                     v51 = iconst.i64 -1
;; @0055                               v40 = iadd v39, v51  ; v51 = -1
;;                                     v50 = iconst.i64 0
;; @0055                               v41 = icmp eq v40, v50  ; v50 = 0
;; @0055                               brif v41, block5, block6
;;
;;                                 block5 cold:
;; @0055                               call fn0(v0, v13)
;; @0055                               jump block7
;;
;;                                 block6:
;; @0055                               v43 = uextend.i64 v13
;; @0055                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @0055                               v44 = load.i64 notrap aligned readonly can_move v48+24
;; @0055                               v45 = iadd v44, v43
;; @0055                               v46 = iconst.i64 8
;; @0055                               v47 = iadd v45, v46  ; v46 = 8
;; @0055                               store.i64 notrap aligned v40, v47
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
;;     gv4 = load.i64 notrap aligned gv3+56
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i64 notrap aligned v0+64
;; @005e                               v5 = ireduce.i32 v4
;; @005e                               v6 = icmp uge v2, v5
;; @005e                               v7 = uextend.i64 v2
;; @005e                               v8 = load.i64 notrap aligned v0+56
;;                                     v63 = iconst.i64 2
;; @005e                               v9 = ishl v7, v63  ; v63 = 2
;; @005e                               v10 = iadd v8, v9
;; @005e                               v11 = iconst.i64 0
;; @005e                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005e                               v13 = load.i32 user5 aligned table v12
;;                                     v62 = iconst.i32 1
;; @005e                               v14 = band v3, v62  ; v62 = 1
;;                                     v61 = iconst.i32 0
;; @005e                               v15 = icmp eq v3, v61  ; v61 = 0
;; @005e                               v16 = uextend.i32 v15
;; @005e                               v17 = bor v14, v16
;; @005e                               brif v17, block3, block2
;;
;;                                 block2:
;; @005e                               v18 = uextend.i64 v3
;; @005e                               v59 = load.i64 notrap aligned readonly can_move v0+8
;; @005e                               v19 = load.i64 notrap aligned readonly can_move v59+24
;; @005e                               v20 = iadd v19, v18
;; @005e                               v21 = iconst.i64 8
;; @005e                               v22 = iadd v20, v21  ; v21 = 8
;; @005e                               v23 = load.i64 notrap aligned v22
;;                                     v58 = iconst.i64 1
;; @005e                               v24 = iadd v23, v58  ; v58 = 1
;; @005e                               v25 = uextend.i64 v3
;; @005e                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @005e                               v26 = load.i64 notrap aligned readonly can_move v56+24
;; @005e                               v27 = iadd v26, v25
;; @005e                               v28 = iconst.i64 8
;; @005e                               v29 = iadd v27, v28  ; v28 = 8
;; @005e                               store notrap aligned v24, v29
;; @005e                               jump block3
;;
;;                                 block3:
;; @005e                               store.i32 user5 aligned table v3, v12
;;                                     v55 = iconst.i32 1
;; @005e                               v30 = band.i32 v13, v55  ; v55 = 1
;;                                     v54 = iconst.i32 0
;; @005e                               v31 = icmp.i32 eq v13, v54  ; v54 = 0
;; @005e                               v32 = uextend.i32 v31
;; @005e                               v33 = bor v30, v32
;; @005e                               brif v33, block7, block4
;;
;;                                 block4:
;; @005e                               v34 = uextend.i64 v13
;; @005e                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @005e                               v35 = load.i64 notrap aligned readonly can_move v52+24
;; @005e                               v36 = iadd v35, v34
;; @005e                               v37 = iconst.i64 8
;; @005e                               v38 = iadd v36, v37  ; v37 = 8
;; @005e                               v39 = load.i64 notrap aligned v38
;;                                     v51 = iconst.i64 -1
;; @005e                               v40 = iadd v39, v51  ; v51 = -1
;;                                     v50 = iconst.i64 0
;; @005e                               v41 = icmp eq v40, v50  ; v50 = 0
;; @005e                               brif v41, block5, block6
;;
;;                                 block5 cold:
;; @005e                               call fn0(v0, v13)
;; @005e                               jump block7
;;
;;                                 block6:
;; @005e                               v43 = uextend.i64 v13
;; @005e                               v48 = load.i64 notrap aligned readonly can_move v0+8
;; @005e                               v44 = load.i64 notrap aligned readonly can_move v48+24
;; @005e                               v45 = iadd v44, v43
;; @005e                               v46 = iconst.i64 8
;; @005e                               v47 = iadd v45, v46  ; v46 = 8
;; @005e                               store.i64 notrap aligned v40, v47
;; @005e                               jump block7
;;
;;                                 block7:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
