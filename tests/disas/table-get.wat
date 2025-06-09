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
;;     gv4 = load.i64 notrap aligned gv3+56
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+64
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+56
;;                                     v61 = iconst.i64 2
;; @0053                               v9 = ishl v7, v61  ; v61 = 2
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 user5 aligned table v12
;;                                     v60 = stack_addr.i64 ss0
;;                                     store notrap v13, v60
;;                                     v59 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v59
;;                                     v58 = iconst.i32 1
;; @0053                               v14 = band v44, v58  ; v58 = 1
;;                                     v57 = stack_addr.i64 ss0
;;                                     v43 = load.i32 notrap v57
;;                                     v56 = iconst.i32 0
;; @0053                               v15 = icmp eq v43, v56  ; v56 = 0
;; @0053                               v16 = uextend.i32 v15
;; @0053                               v17 = bor v14, v16
;; @0053                               brif v17, block5, block2
;;
;;                                 block2:
;; @0053                               v19 = load.i64 notrap aligned readonly v0+40
;; @0053                               v20 = load.i64 notrap aligned v19
;; @0053                               v21 = load.i64 notrap aligned v19+8
;; @0053                               v22 = icmp eq v20, v21
;; @0053                               brif v22, block3, block4
;;
;;                                 block4:
;;                                     v55 = stack_addr.i64 ss0
;;                                     v42 = load.i32 notrap v55
;; @0053                               v23 = uextend.i64 v42
;; @0053                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v24 = load.i64 notrap aligned readonly can_move v53+24
;; @0053                               v25 = iadd v24, v23
;; @0053                               v26 = iconst.i64 8
;; @0053                               v27 = iadd v25, v26  ; v26 = 8
;; @0053                               v28 = load.i64 notrap aligned v27
;;                                     v52 = iconst.i64 1
;; @0053                               v29 = iadd v28, v52  ; v52 = 1
;;                                     v51 = stack_addr.i64 ss0
;;                                     v41 = load.i32 notrap v51
;; @0053                               v30 = uextend.i64 v41
;; @0053                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v31 = load.i64 notrap aligned readonly can_move v49+24
;; @0053                               v32 = iadd v31, v30
;; @0053                               v33 = iconst.i64 8
;; @0053                               v34 = iadd v32, v33  ; v33 = 8
;; @0053                               store notrap aligned v29, v34
;;                                     v48 = stack_addr.i64 ss0
;;                                     v40 = load.i32 notrap v48
;; @0053                               store notrap aligned v40, v20
;;                                     v47 = iconst.i64 4
;; @0053                               v35 = iadd.i64 v20, v47  ; v47 = 4
;; @0053                               store notrap aligned v35, v19
;; @0053                               jump block5
;;
;;                                 block3 cold:
;;                                     v46 = stack_addr.i64 ss0
;;                                     v39 = load.i32 notrap v46
;; @0053                               v37 = call fn0(v0, v39), stack_map=[i32 @ ss0+0]
;; @0053                               jump block5
;;
;;                                 block5:
;;                                     v45 = stack_addr.i64 ss0
;;                                     v38 = load.i32 notrap v45
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v38
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+56
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+64
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+56
;;                                     v61 = iconst.i64 2
;; @005a                               v9 = ishl v7, v61  ; v61 = 2
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 user5 aligned table v12
;;                                     v60 = stack_addr.i64 ss0
;;                                     store notrap v13, v60
;;                                     v59 = stack_addr.i64 ss0
;;                                     v44 = load.i32 notrap v59
;;                                     v58 = iconst.i32 1
;; @005a                               v14 = band v44, v58  ; v58 = 1
;;                                     v57 = stack_addr.i64 ss0
;;                                     v43 = load.i32 notrap v57
;;                                     v56 = iconst.i32 0
;; @005a                               v15 = icmp eq v43, v56  ; v56 = 0
;; @005a                               v16 = uextend.i32 v15
;; @005a                               v17 = bor v14, v16
;; @005a                               brif v17, block5, block2
;;
;;                                 block2:
;; @005a                               v19 = load.i64 notrap aligned readonly v0+40
;; @005a                               v20 = load.i64 notrap aligned v19
;; @005a                               v21 = load.i64 notrap aligned v19+8
;; @005a                               v22 = icmp eq v20, v21
;; @005a                               brif v22, block3, block4
;;
;;                                 block4:
;;                                     v55 = stack_addr.i64 ss0
;;                                     v42 = load.i32 notrap v55
;; @005a                               v23 = uextend.i64 v42
;; @005a                               v53 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v24 = load.i64 notrap aligned readonly can_move v53+24
;; @005a                               v25 = iadd v24, v23
;; @005a                               v26 = iconst.i64 8
;; @005a                               v27 = iadd v25, v26  ; v26 = 8
;; @005a                               v28 = load.i64 notrap aligned v27
;;                                     v52 = iconst.i64 1
;; @005a                               v29 = iadd v28, v52  ; v52 = 1
;;                                     v51 = stack_addr.i64 ss0
;;                                     v41 = load.i32 notrap v51
;; @005a                               v30 = uextend.i64 v41
;; @005a                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v31 = load.i64 notrap aligned readonly can_move v49+24
;; @005a                               v32 = iadd v31, v30
;; @005a                               v33 = iconst.i64 8
;; @005a                               v34 = iadd v32, v33  ; v33 = 8
;; @005a                               store notrap aligned v29, v34
;;                                     v48 = stack_addr.i64 ss0
;;                                     v40 = load.i32 notrap v48
;; @005a                               store notrap aligned v40, v20
;;                                     v47 = iconst.i64 4
;; @005a                               v35 = iadd.i64 v20, v47  ; v47 = 4
;; @005a                               store notrap aligned v35, v19
;; @005a                               jump block5
;;
;;                                 block3 cold:
;;                                     v46 = stack_addr.i64 ss0
;;                                     v39 = load.i32 notrap v46
;; @005a                               v37 = call fn0(v0, v39), stack_map=[i32 @ ss0+0]
;; @005a                               jump block5
;;
;;                                 block5:
;;                                     v45 = stack_addr.i64 ss0
;;                                     v38 = load.i32 notrap v45
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v38
;; }
