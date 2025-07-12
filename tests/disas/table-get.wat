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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+56
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+48
;;                                     v62 = iconst.i64 2
;; @0053                               v9 = ishl v7, v62  ; v62 = 2
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = iconst.i64 0
;; @0053                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0053                               v13 = load.i32 user5 aligned table v12
;;                                     v61 = iconst.i32 1
;; @0053                               v14 = band v13, v61  ; v61 = 1
;;                                     v60 = iconst.i32 0
;; @0053                               v15 = icmp eq v13, v60  ; v60 = 0
;; @0053                               v16 = uextend.i32 v15
;; @0053                               v17 = bor v14, v16
;; @0053                               brif v17, block4, block2
;;
;;                                 block2:
;; @0053                               v18 = uextend.i64 v13
;; @0053                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v19 = load.i64 notrap aligned readonly can_move v58+24
;; @0053                               v20 = iadd v19, v18
;; @0053                               v21 = load.i32 notrap aligned v20
;; @0053                               v22 = iconst.i32 2
;; @0053                               v23 = band v21, v22  ; v22 = 2
;; @0053                               brif v23, block4, block3
;;
;;                                 block3:
;; @0053                               v25 = load.i64 notrap aligned readonly v0+32
;; @0053                               v26 = load.i32 notrap aligned v25
;; @0053                               v27 = uextend.i64 v13
;; @0053                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v28 = load.i64 notrap aligned readonly can_move v56+24
;; @0053                               v29 = iadd v28, v27
;; @0053                               v30 = iconst.i64 16
;; @0053                               v31 = iadd v29, v30  ; v30 = 16
;; @0053                               store notrap aligned v26, v31
;; @0053                               v32 = iconst.i32 2
;; @0053                               v33 = bor.i32 v21, v32  ; v32 = 2
;; @0053                               v34 = uextend.i64 v13
;; @0053                               v54 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v35 = load.i64 notrap aligned readonly can_move v54+24
;; @0053                               v36 = iadd v35, v34
;; @0053                               store notrap aligned v33, v36
;; @0053                               v37 = uextend.i64 v13
;; @0053                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v38 = load.i64 notrap aligned readonly can_move v52+24
;; @0053                               v39 = iadd v38, v37
;; @0053                               v40 = iconst.i64 8
;; @0053                               v41 = iadd v39, v40  ; v40 = 8
;; @0053                               v42 = load.i64 notrap aligned v41
;;                                     v51 = iconst.i64 1
;; @0053                               v43 = iadd v42, v51  ; v51 = 1
;; @0053                               v44 = uextend.i64 v13
;; @0053                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @0053                               v45 = load.i64 notrap aligned readonly can_move v49+24
;; @0053                               v46 = iadd v45, v44
;; @0053                               v47 = iconst.i64 8
;; @0053                               v48 = iadd v46, v47  ; v47 = 8
;; @0053                               store notrap aligned v43, v48
;; @0053                               store.i32 notrap aligned v13, v25
;; @0053                               jump block4
;;
;;                                 block4:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v13
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv7 = load.i64 notrap aligned readonly can_move gv6+24
;;     gv8 = load.i64 notrap aligned gv6+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+56
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+48
;;                                     v62 = iconst.i64 2
;; @005a                               v9 = ishl v7, v62  ; v62 = 2
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = iconst.i64 0
;; @005a                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005a                               v13 = load.i32 user5 aligned table v12
;;                                     v61 = iconst.i32 1
;; @005a                               v14 = band v13, v61  ; v61 = 1
;;                                     v60 = iconst.i32 0
;; @005a                               v15 = icmp eq v13, v60  ; v60 = 0
;; @005a                               v16 = uextend.i32 v15
;; @005a                               v17 = bor v14, v16
;; @005a                               brif v17, block4, block2
;;
;;                                 block2:
;; @005a                               v18 = uextend.i64 v13
;; @005a                               v58 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v19 = load.i64 notrap aligned readonly can_move v58+24
;; @005a                               v20 = iadd v19, v18
;; @005a                               v21 = load.i32 notrap aligned v20
;; @005a                               v22 = iconst.i32 2
;; @005a                               v23 = band v21, v22  ; v22 = 2
;; @005a                               brif v23, block4, block3
;;
;;                                 block3:
;; @005a                               v25 = load.i64 notrap aligned readonly v0+32
;; @005a                               v26 = load.i32 notrap aligned v25
;; @005a                               v27 = uextend.i64 v13
;; @005a                               v56 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v28 = load.i64 notrap aligned readonly can_move v56+24
;; @005a                               v29 = iadd v28, v27
;; @005a                               v30 = iconst.i64 16
;; @005a                               v31 = iadd v29, v30  ; v30 = 16
;; @005a                               store notrap aligned v26, v31
;; @005a                               v32 = iconst.i32 2
;; @005a                               v33 = bor.i32 v21, v32  ; v32 = 2
;; @005a                               v34 = uextend.i64 v13
;; @005a                               v54 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v35 = load.i64 notrap aligned readonly can_move v54+24
;; @005a                               v36 = iadd v35, v34
;; @005a                               store notrap aligned v33, v36
;; @005a                               v37 = uextend.i64 v13
;; @005a                               v52 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v38 = load.i64 notrap aligned readonly can_move v52+24
;; @005a                               v39 = iadd v38, v37
;; @005a                               v40 = iconst.i64 8
;; @005a                               v41 = iadd v39, v40  ; v40 = 8
;; @005a                               v42 = load.i64 notrap aligned v41
;;                                     v51 = iconst.i64 1
;; @005a                               v43 = iadd v42, v51  ; v51 = 1
;; @005a                               v44 = uextend.i64 v13
;; @005a                               v49 = load.i64 notrap aligned readonly can_move v0+8
;; @005a                               v45 = load.i64 notrap aligned readonly can_move v49+24
;; @005a                               v46 = iadd v45, v44
;; @005a                               v47 = iconst.i64 8
;; @005a                               v48 = iadd v46, v47  ; v47 = 8
;; @005a                               store notrap aligned v43, v48
;; @005a                               store.i32 notrap aligned v13, v25
;; @005a                               jump block4
;;
;;                                 block4:
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v13
;; }
