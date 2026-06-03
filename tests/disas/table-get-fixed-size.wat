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

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0052                               v3 = iconst.i32 0
;; @0054                               v4 = iconst.i32 7
;; @0054                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0054                               v6 = uextend.i64 v3  ; v3 = 0
;; @0054                               v7 = load.i64 notrap aligned readonly can_move v0+48
;; @0054                               v8 = iconst.i64 2
;; @0054                               v9 = ishl v6, v8  ; v8 = 2
;; @0054                               v10 = iadd v7, v9
;; @0054                               v11 = iconst.i64 0
;; @0054                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @0054                               v13 = load.i32 user6 aligned region0 v12
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v13
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 1073741824 "PublicTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005b                               v4 = iconst.i32 7
;; @005b                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005b                               v6 = uextend.i64 v2
;; @005b                               v7 = load.i64 notrap aligned readonly can_move v0+48
;; @005b                               v8 = iconst.i64 2
;; @005b                               v9 = ishl v6, v8  ; v8 = 2
;; @005b                               v10 = iadd v7, v9
;; @005b                               v11 = iconst.i64 0
;; @005b                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @005b                               v13 = load.i32 user6 aligned region0 v12
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v13
;; }
