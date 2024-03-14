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

;; function u0:0(i64 vmctx, i64) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, r64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v14 -> v0
;;                                     v19 -> v0
;;                                     v25 -> v0
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               brif v5, block6, block7
;;
;;                                 block6 cold:
;; @0054                               trap table_oob
;;
;;                                 block7:
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned v25+72
;;                                     v26 = iconst.i64 3
;; @0054                               v8 = ishl v6, v26  ; v26 = 3
;; @0054                               v9 = iadd v7, v8
;; @0054                               v10 = icmp.i32 uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v11 = select_spectre_guard v10, v7, v9
;; @0054                               v12 = load.r64 notrap aligned table v11
;;                                     v2 -> v12
;; @0054                               v13 = is_null v12
;; @0054                               brif v13, block2, block3
;;
;;                                 block3:
;; @0054                               v15 = load.i64 notrap aligned v14+32
;; @0054                               v16 = load.i64 notrap aligned v15
;; @0054                               v17 = load.i64 notrap aligned v15+8
;; @0054                               v18 = icmp eq v16, v17
;; @0054                               brif v18, block4, block5
;;
;;                                 block5:
;; @0054                               v22 = load.i64 notrap aligned v12
;;                                     v27 = iconst.i64 1
;; @0054                               v23 = iadd v22, v27  ; v27 = 1
;; @0054                               store notrap aligned v23, v12
;; @0054                               store.r64 notrap aligned v12, v16
;;                                     v28 = iconst.i64 8
;; @0054                               v24 = iadd.i64 v16, v28  ; v28 = 8
;; @0054                               store notrap aligned v24, v15
;; @0054                               jump block2
;;
;;                                 block4:
;; @0054                               v20 = load.i64 notrap aligned readonly v19+56
;; @0054                               v21 = load.i64 notrap aligned readonly v20+208
;; @0054                               call_indirect sig0, v21(v19, v12)
;; @0054                               jump block2
;;
;;                                 block2:
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, r64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v14 -> v0
;;                                     v19 -> v0
;;                                     v25 -> v0
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               brif v5, block6, block7
;;
;;                                 block6 cold:
;; @005b                               trap table_oob
;;
;;                                 block7:
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned v25+72
;;                                     v26 = iconst.i64 3
;; @005b                               v8 = ishl v6, v26  ; v26 = 3
;; @005b                               v9 = iadd v7, v8
;; @005b                               v10 = icmp.i32 uge v2, v4  ; v4 = 7
;; @005b                               v11 = select_spectre_guard v10, v7, v9
;; @005b                               v12 = load.r64 notrap aligned table v11
;;                                     v3 -> v12
;; @005b                               v13 = is_null v12
;; @005b                               brif v13, block2, block3
;;
;;                                 block3:
;; @005b                               v15 = load.i64 notrap aligned v14+32
;; @005b                               v16 = load.i64 notrap aligned v15
;; @005b                               v17 = load.i64 notrap aligned v15+8
;; @005b                               v18 = icmp eq v16, v17
;; @005b                               brif v18, block4, block5
;;
;;                                 block5:
;; @005b                               v22 = load.i64 notrap aligned v12
;;                                     v27 = iconst.i64 1
;; @005b                               v23 = iadd v22, v27  ; v27 = 1
;; @005b                               store notrap aligned v23, v12
;; @005b                               store.r64 notrap aligned v12, v16
;;                                     v28 = iconst.i64 8
;; @005b                               v24 = iadd.i64 v16, v28  ; v28 = 8
;; @005b                               store notrap aligned v24, v15
;; @005b                               jump block2
;;
;;                                 block4:
;; @005b                               v20 = load.i64 notrap aligned readonly v19+56
;; @005b                               v21 = load.i64 notrap aligned readonly v20+208
;; @005b                               call_indirect sig0, v21(v19, v12)
;; @005b                               jump block2
;;
;;                                 block2:
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v3
;; }
