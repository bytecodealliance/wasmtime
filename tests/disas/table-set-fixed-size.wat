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
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 1 "table"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v3 = iconst.i32 0
;; @0056                               v4 = iconst.i32 7
;; @0056                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 7
;; @0056                               v6 = uextend.i64 v3  ; v3 = 0
;; @0056                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v12 = iconst.i64 2
;; @0056                               v8 = ishl v6, v12  ; v12 = 2
;; @0056                               v9 = iadd v7, v8
;; @0056                               v10 = iconst.i64 0
;; @0056                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @0056                               store user6 aligned region0 v2, v11
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 1 "table"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @005f                               v4 = iconst.i32 7
;; @005f                               v5 = icmp uge v2, v4  ; v4 = 7
;; @005f                               v6 = uextend.i64 v2
;; @005f                               v7 = load.i64 notrap aligned readonly can_move v0+48
;;                                     v12 = iconst.i64 2
;; @005f                               v8 = ishl v6, v12  ; v12 = 2
;; @005f                               v9 = iadd v7, v8
;; @005f                               v10 = iconst.i64 0
;; @005f                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @005f                               store user6 aligned region0 v3, v11
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
