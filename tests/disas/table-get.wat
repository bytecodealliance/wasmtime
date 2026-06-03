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
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0051                               v3 = iconst.i32 0
;; @0053                               v4 = load.i64 notrap aligned v0+56
;; @0053                               v5 = ireduce.i32 v4
;; @0053                               v6 = icmp uge v3, v5  ; v3 = 0
;; @0053                               v7 = uextend.i64 v3  ; v3 = 0
;; @0053                               v8 = load.i64 notrap aligned v0+48
;; @0053                               v9 = iconst.i64 2
;; @0053                               v10 = ishl v7, v9  ; v9 = 2
;; @0053                               v11 = iadd v8, v10
;; @0053                               v12 = iconst.i64 0
;; @0053                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @0053                               v14 = load.i32 user6 aligned region0 v13
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v14
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+48
;;     gv5 = load.i64 notrap aligned gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005a                               v4 = load.i64 notrap aligned v0+56
;; @005a                               v5 = ireduce.i32 v4
;; @005a                               v6 = icmp uge v2, v5
;; @005a                               v7 = uextend.i64 v2
;; @005a                               v8 = load.i64 notrap aligned v0+48
;; @005a                               v9 = iconst.i64 2
;; @005a                               v10 = ishl v7, v9  ; v9 = 2
;; @005a                               v11 = iadd v8, v10
;; @005a                               v12 = iconst.i64 0
;; @005a                               v13 = select_spectre_guard v6, v12, v11  ; v12 = 0
;; @005a                               v14 = load.i32 user6 aligned region0 v13
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v14
;; }
