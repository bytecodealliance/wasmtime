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

;; function u0:0(i64 vmctx, i64, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i32 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: r64):
;;                                     v20 -> v0
;;                                     v23 -> v0
;;                                     v24 -> v0
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i32 notrap aligned v23+80
;; @0055                               v5 = icmp uge v3, v4  ; v3 = 0
;; @0055                               brif v5, block7, block8
;;
;;                                 block7 cold:
;; @0055                               trap table_oob
;;
;;                                 block8:
;; @0055                               v6 = uextend.i64 v3  ; v3 = 0
;; @0055                               v7 = load.i64 notrap aligned v24+72
;;                                     v25 = iconst.i64 3
;; @0055                               v8 = ishl v6, v25  ; v25 = 3
;; @0055                               v9 = iadd v7, v8
;; @0055                               v10 = icmp.i32 uge v3, v4  ; v3 = 0
;; @0055                               v11 = select_spectre_guard v10, v7, v9
;; @0055                               v12 = load.i64 notrap aligned table v11
;; @0055                               store.r64 notrap aligned table v2, v11
;; @0055                               v13 = is_null.r64 v2
;; @0055                               brif v13, block3, block2
;;
;;                                 block2:
;; @0055                               v14 = load.i64 notrap aligned v2
;;                                     v26 = iconst.i64 1
;; @0055                               v15 = iadd v14, v26  ; v26 = 1
;; @0055                               store notrap aligned v15, v2
;; @0055                               jump block3
;;
;;                                 block3:
;;                                     v27 = iconst.i64 0
;; @0055                               v16 = icmp.i64 eq v12, v27  ; v27 = 0
;; @0055                               brif v16, block6, block4
;;
;;                                 block4:
;; @0055                               v17 = load.i64 notrap aligned v12
;;                                     v28 = iconst.i64 -1
;; @0055                               v18 = iadd v17, v28  ; v28 = -1
;; @0055                               store notrap aligned v18, v12
;;                                     v29 = iconst.i64 0
;; @0055                               v19 = icmp eq v18, v29  ; v29 = 0
;; @0055                               brif v19, block5, block6
;;
;;                                 block5:
;; @0055                               v21 = load.i64 notrap aligned readonly v20+56
;; @0055                               v22 = load.i64 notrap aligned readonly v21+200
;; @0055                               call_indirect sig0, v22(v20, v12)
;; @0055                               jump block6
;;
;;                                 block6:
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     gv5 = load.i32 notrap aligned gv3+80
;;     sig0 = (i64 vmctx, i64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: r64):
;;                                     v20 -> v0
;;                                     v23 -> v0
;;                                     v24 -> v0
;; @005e                               v4 = load.i32 notrap aligned v23+80
;; @005e                               v5 = icmp uge v2, v4
;; @005e                               brif v5, block7, block8
;;
;;                                 block7 cold:
;; @005e                               trap table_oob
;;
;;                                 block8:
;; @005e                               v6 = uextend.i64 v2
;; @005e                               v7 = load.i64 notrap aligned v24+72
;;                                     v25 = iconst.i64 3
;; @005e                               v8 = ishl v6, v25  ; v25 = 3
;; @005e                               v9 = iadd v7, v8
;; @005e                               v10 = icmp.i32 uge v2, v4
;; @005e                               v11 = select_spectre_guard v10, v7, v9
;; @005e                               v12 = load.i64 notrap aligned table v11
;; @005e                               store.r64 notrap aligned table v3, v11
;; @005e                               v13 = is_null.r64 v3
;; @005e                               brif v13, block3, block2
;;
;;                                 block2:
;; @005e                               v14 = load.i64 notrap aligned v3
;;                                     v26 = iconst.i64 1
;; @005e                               v15 = iadd v14, v26  ; v26 = 1
;; @005e                               store notrap aligned v15, v3
;; @005e                               jump block3
;;
;;                                 block3:
;;                                     v27 = iconst.i64 0
;; @005e                               v16 = icmp.i64 eq v12, v27  ; v27 = 0
;; @005e                               brif v16, block6, block4
;;
;;                                 block4:
;; @005e                               v17 = load.i64 notrap aligned v12
;;                                     v28 = iconst.i64 -1
;; @005e                               v18 = iadd v17, v28  ; v28 = -1
;; @005e                               store notrap aligned v18, v12
;;                                     v29 = iconst.i64 0
;; @005e                               v19 = icmp eq v18, v29  ; v29 = 0
;; @005e                               brif v19, block5, block6
;;
;;                                 block5:
;; @005e                               v21 = load.i64 notrap aligned readonly v20+56
;; @005e                               v22 = load.i64 notrap aligned readonly v21+200
;; @005e                               call_indirect sig0, v22(v20, v12)
;; @005e                               jump block6
;;
;;                                 block6:
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
