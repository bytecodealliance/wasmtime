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
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i32 notrap aligned v0+80
;; @0053                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0053                               v6 = uextend.i64 v3  ; v3 = 0
;; @0053                               v7 = load.i64 notrap aligned v0+72
;;                                     v25 = iconst.i64 3
;; @0053                               v8 = ishl v6, v25  ; v25 = 3
;; @0053                               v9 = iadd v7, v8
;; @0053                               v10 = iconst.i64 0
;; @0053                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0053                               v12 = load.r64 table_oob aligned table v11
;; @0053                               v13 = is_null v12
;; @0053                               brif v13, block2, block3
;;
;;                                 block3:
;; @0053                               v15 = load.i64 notrap aligned v0+40
;; @0053                               v16 = load.i64 notrap aligned v15
;; @0053                               v17 = load.i64 notrap aligned v15+8
;; @0053                               v18 = icmp eq v16, v17
;; @0053                               brif v18, block4, block5
;;
;;                                 block5:
;; @0053                               v20 = load.i64 notrap aligned v12
;;                                     v26 = iconst.i64 1
;; @0053                               v21 = iadd v20, v26  ; v26 = 1
;; @0053                               store notrap aligned v21, v12
;; @0053                               store.r64 notrap aligned v12, v16
;;                                     v27 = iconst.i64 8
;; @0053                               v22 = iadd.i64 v16, v27  ; v27 = 8
;; @0053                               store notrap aligned v22, v15
;; @0053                               jump block2
;;
;;                                 block4:
;; @0053                               call fn0(v0, v12)
;; @0053                               jump block2
;;
;;                                 block2:
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v12
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
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i32 notrap aligned v0+80
;; @005a                               v5 = icmp uge v2, v4
;; @005a                               v6 = uextend.i64 v2
;; @005a                               v7 = load.i64 notrap aligned v0+72
;;                                     v25 = iconst.i64 3
;; @005a                               v8 = ishl v6, v25  ; v25 = 3
;; @005a                               v9 = iadd v7, v8
;; @005a                               v10 = iconst.i64 0
;; @005a                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005a                               v12 = load.r64 table_oob aligned table v11
;; @005a                               v13 = is_null v12
;; @005a                               brif v13, block2, block3
;;
;;                                 block3:
;; @005a                               v15 = load.i64 notrap aligned v0+40
;; @005a                               v16 = load.i64 notrap aligned v15
;; @005a                               v17 = load.i64 notrap aligned v15+8
;; @005a                               v18 = icmp eq v16, v17
;; @005a                               brif v18, block4, block5
;;
;;                                 block5:
;; @005a                               v20 = load.i64 notrap aligned v12
;;                                     v26 = iconst.i64 1
;; @005a                               v21 = iadd v20, v26  ; v26 = 1
;; @005a                               store notrap aligned v21, v12
;; @005a                               store.r64 notrap aligned v12, v16
;;                                     v27 = iconst.i64 8
;; @005a                               v22 = iadd.i64 v16, v27  ; v27 = 8
;; @005a                               store notrap aligned v22, v15
;; @005a                               jump block2
;;
;;                                 block4:
;; @005a                               call fn0(v0, v12)
;; @005a                               jump block2
;;
;;                                 block2:
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v12
;; }
