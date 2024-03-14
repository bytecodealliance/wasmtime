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

;; function u0:0(i64 vmctx, i64, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: r64):
;;                                     v20 -> v0
;;                                     v23 -> v0
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               brif v5, block7, block8
;;
;;                                 block7 cold:
;; @0056                               trap table_oob
;;
;;                                 block8:
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned v23+72
;;                                     v24 = iconst.i64 3
;; @0056                               v8 = ishl v6, v24  ; v24 = 3
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = icmp.i32 uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v11 = select_spectre_guard v10, v7, v9
;; @0056                               v12 = load.i64 notrap aligned table v11
;; @0056                               store.r64 notrap aligned table v2, v11
;; @0056                               v13 = is_null.r64 v2
;; @0056                               brif v13, block3, block2
;;
;;                                 block2:
;; @0056                               v14 = load.i64 notrap aligned v2
;;                                     v25 = iconst.i64 1
;; @0056                               v15 = iadd v14, v25  ; v25 = 1
;; @0056                               store notrap aligned v15, v2
;; @0056                               jump block3
;;
;;                                 block3:
;;                                     v26 = iconst.i64 0
;; @0056                               v16 = icmp.i64 eq v12, v26  ; v26 = 0
;; @0056                               brif v16, block6, block4
;;
;;                                 block4:
;; @0056                               v17 = load.i64 notrap aligned v12
;;                                     v27 = iconst.i64 -1
;; @0056                               v18 = iadd v17, v27  ; v27 = -1
;; @0056                               store notrap aligned v18, v12
;;                                     v28 = iconst.i64 0
;; @0056                               v19 = icmp eq v18, v28  ; v28 = 0
;; @0056                               brif v19, block5, block6
;;
;;                                 block5:
;; @0056                               v21 = load.i64 notrap aligned readonly v20+56
;; @0056                               v22 = load.i64 notrap aligned readonly v21+200
;; @0056                               call_indirect sig0, v22(v20, v12)
;; @0056                               jump block6
;;
;;                                 block6:
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, r64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64) system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: r64):
;;                                     v20 -> v0
;;                                     v23 -> v0
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               brif v5, block7, block8
;;
;;                                 block7 cold:
;; @005f                               trap table_oob
;;
;;                                 block8:
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned v23+72
;;                                     v24 = iconst.i64 3
;; @005f                               v8 = ishl v6, v24  ; v24 = 3
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = icmp.i32 uge v2, v4  ; v4 = 7
;; @005f                               v11 = select_spectre_guard v10, v7, v9
;; @005f                               v12 = load.i64 notrap aligned table v11
;; @005f                               store.r64 notrap aligned table v3, v11
;; @005f                               v13 = is_null.r64 v3
;; @005f                               brif v13, block3, block2
;;
;;                                 block2:
;; @005f                               v14 = load.i64 notrap aligned v3
;;                                     v25 = iconst.i64 1
;; @005f                               v15 = iadd v14, v25  ; v25 = 1
;; @005f                               store notrap aligned v15, v3
;; @005f                               jump block3
;;
;;                                 block3:
;;                                     v26 = iconst.i64 0
;; @005f                               v16 = icmp.i64 eq v12, v26  ; v26 = 0
;; @005f                               brif v16, block6, block4
;;
;;                                 block4:
;; @005f                               v17 = load.i64 notrap aligned v12
;;                                     v27 = iconst.i64 -1
;; @005f                               v18 = iadd v17, v27  ; v27 = -1
;; @005f                               store notrap aligned v18, v12
;;                                     v28 = iconst.i64 0
;; @005f                               v19 = icmp eq v18, v28  ; v28 = 0
;; @005f                               brif v19, block5, block6
;;
;;                                 block5:
;; @005f                               v21 = load.i64 notrap aligned readonly v20+56
;; @005f                               v22 = load.i64 notrap aligned readonly v21+200
;; @005f                               call_indirect sig0, v22(v20, v12)
;; @005f                               jump block6
;;
;;                                 block6:
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
