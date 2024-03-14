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

;; function u0:0(i64 vmctx, i64) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i32 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, r64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v14 -> v0
;;                                     v19 -> v0
;;                                     v25 -> v0
;;                                     v26 -> v0
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i32 notrap aligned v25+80
;; @0053                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0053                               brif v5, block6, block7
;;
;;                                 block6 cold:
;; @0053                               trap table_oob
;;
;;                                 block7:
;; @0053                               v6 = uextend.i64 v3  ; v3 = 0
;; @0053                               v7 = load.i64 notrap aligned v26+72
;;                                     v27 = iconst.i64 3
;; @0053                               v8 = ishl v6, v27  ; v27 = 3
;; @0053                               v9 = iadd v7, v8
;; @0053                               v10 = icmp.i32 uge v3, v4  ; v3 = 0
;; @0053                               v11 = select_spectre_guard v10, v7, v9
;; @0053                               v12 = load.r64 notrap aligned table v11
;;                                     v2 -> v12
;; @0053                               v13 = is_null v12
;; @0053                               brif v13, block2, block3
;;
;;                                 block3:
;; @0053                               v15 = load.i64 notrap aligned v14+32
;; @0053                               v16 = load.i64 notrap aligned v15
;; @0053                               v17 = load.i64 notrap aligned v15+8
;; @0053                               v18 = icmp eq v16, v17
;; @0053                               brif v18, block4, block5
;;
;;                                 block5:
;; @0053                               v22 = load.i64 notrap aligned v12
;;                                     v28 = iconst.i64 1
;; @0053                               v23 = iadd v22, v28  ; v28 = 1
;; @0053                               store notrap aligned v23, v12
;; @0053                               store.r64 notrap aligned v12, v16
;;                                     v29 = iconst.i64 8
;; @0053                               v24 = iadd.i64 v16, v29  ; v29 = 8
;; @0053                               store notrap aligned v24, v15
;; @0053                               jump block2
;;
;;                                 block4:
;; @0053                               v20 = load.i64 notrap aligned readonly v19+56
;; @0053                               v21 = load.i64 notrap aligned readonly v20+208
;; @0053                               call_indirect sig0, v21(v19, v12)
;; @0053                               jump block2
;;
;;                                 block2:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i32 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, r64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v14 -> v0
;;                                     v19 -> v0
;;                                     v25 -> v0
;;                                     v26 -> v0
;; @005a                               v4 = load.i32 notrap aligned v25+80
;; @005a                               v5 = icmp uge v2, v4
;; @005a                               brif v5, block6, block7
;;
;;                                 block6 cold:
;; @005a                               trap table_oob
;;
;;                                 block7:
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v7 = load.i64 notrap aligned v26+72
;;                                     v27 = iconst.i64 3
;; @005a                               v8 = ishl v6, v27  ; v27 = 3
;; @005a                               v9 = iadd v7, v8
;; @005a                               v10 = icmp.i32 uge v2, v4
;; @005a                               v11 = select_spectre_guard v10, v7, v9
;; @005a                               v12 = load.r64 notrap aligned table v11
;;                                     v3 -> v12
;; @005a                               v13 = is_null v12
;; @005a                               brif v13, block2, block3
;;
;;                                 block3:
;; @005a                               v15 = load.i64 notrap aligned v14+32
;; @005a                               v16 = load.i64 notrap aligned v15
;; @005a                               v17 = load.i64 notrap aligned v15+8
;; @005a                               v18 = icmp eq v16, v17
;; @005a                               brif v18, block4, block5
;;
;;                                 block5:
;; @005a                               v22 = load.i64 notrap aligned v12
;;                                     v28 = iconst.i64 1
;; @005a                               v23 = iadd v22, v28  ; v28 = 1
;; @005a                               store notrap aligned v23, v12
;; @005a                               store.r64 notrap aligned v12, v16
;;                                     v29 = iconst.i64 8
;; @005a                               v24 = iadd.i64 v16, v29  ; v29 = 8
;; @005a                               store notrap aligned v24, v15
;; @005a                               jump block2
;;
;;                                 block4:
;; @005a                               v20 = load.i64 notrap aligned readonly v19+56
;; @005a                               v21 = load.i64 notrap aligned readonly v20+208
;; @005a                               call_indirect sig0, v21(v19, v12)
;; @005a                               jump block2
;;
;;                                 block2:
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v3
;; }
