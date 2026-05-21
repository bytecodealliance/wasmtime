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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0051                               v3 = iconst.i32 0
;; @0055                               v4 = load.i64 notrap aligned v0+56
;; @0055                               v5 = ireduce.i32 v4
;; @0055                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0055                               v7 = uextend.i64 v3  ; v3 = 0
;; @0055                               v8 = load.i64 notrap aligned v0+48
;;                                     v13 = iconst.i64 2
;; @0055                               v9 = ishl v7, v13  ; v13 = 2
;; @0055                               v10 = iadd v8, v9
;; @0055                               v11 = iconst.i64 0
;; @0055                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @0055                               store user6 aligned table v2, v12
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005e                               v4 = load.i64 notrap aligned v0+56
;; @005e                               v5 = ireduce.i32 v4
;; @005e                               v6 = icmp uge v2, v5
;; @005e                               v7 = uextend.i64 v2
;; @005e                               v8 = load.i64 notrap aligned v0+48
;;                                     v13 = iconst.i64 2
;; @005e                               v9 = ishl v7, v13  ; v13 = 2
;; @005e                               v10 = iadd v8, v9
;; @005e                               v11 = iconst.i64 0
;; @005e                               v12 = select_spectre_guard v6, v11, v10  ; v11 = 0
;; @005e                               store user6 aligned table v3, v12
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
