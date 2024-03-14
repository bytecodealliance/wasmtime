;;! target = "x86_64"
;;! optimize = true

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimization but with opt-level "none" to
;; legalize away macro instructions.

(module
  (table (export "table") 7 7 externref)
  (func (export "table.get.const") (result externref)
    i32.const 0
    table.get 0)
  (func (export "table.get.var") (param i32) (result externref)
    local.get 0
    table.get 0))

;; function u0:0(i64 vmctx) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i64):
;;                                     v12 -> v0
;; @0052                               v2 = iconst.i32 0
;; @0054                               v3 = iconst.i32 7
;; @0054                               v4 = icmp uge v2, v3  ; v2 = 0, v3 = 7
;; @0054                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @0054                               trap table_oob
;;
;;                                 block3:
;; @0054                               v5 = uextend.i64 v2  ; v2 = 0
;; @0054                               v6 = load.i64 notrap aligned readonly v12
;;                                     v13 = iconst.i64 4
;; @0054                               v7 = ishl v5, v13  ; v13 = 4
;; @0054                               v8 = iadd v6, v7
;; @0054                               v9 = icmp.i32 uge v2, v3  ; v2 = 0, v3 = 7
;; @0054                               v10 = select_spectre_guard v9, v6, v8
;; @0054                               v11 = load.r64 notrap aligned table v10
;;                                     v1 -> v11
;; @0056                               jump block1
;;
;;                                 block1:
;; @0056                               return v1
;; }
;;
;; function u0:1(i32, i64 vmctx) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;;                                     v12 -> v1
;; @005b                               v3 = iconst.i32 7
;; @005b                               v4 = icmp uge v0, v3  ; v3 = 7
;; @005b                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @005b                               trap table_oob
;;
;;                                 block3:
;; @005b                               v5 = uextend.i64 v0
;; @005b                               v6 = load.i64 notrap aligned readonly v12
;;                                     v13 = iconst.i64 4
;; @005b                               v7 = ishl v5, v13  ; v13 = 4
;; @005b                               v8 = iadd v6, v7
;; @005b                               v9 = icmp.i32 uge v0, v3  ; v3 = 7
;; @005b                               v10 = select_spectre_guard v9, v6, v8
;; @005b                               v11 = load.r64 notrap aligned table v10
;;                                     v2 -> v11
;; @005d                               jump block1
;;
;;                                 block1:
;; @005d                               return v2
;; }
