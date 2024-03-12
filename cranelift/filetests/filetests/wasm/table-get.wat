;;! target = "x86_64"
;;! optimize = true

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with opt-level "none" to legalize away table_addr instructions.

(module
  (table (export "table") 1 externref)
  (func (export "table.get.const") (result externref)
    i32.const 0
    table.get 0)
  (func (export "table.get.var") (param i32) (result externref)
    local.get 0
    table.get 0))

;; function u0:0(i64 vmctx) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;     gv2 = load.i32 notrap aligned readonly gv0
;;     table0 = dynamic gv1, min 0, bound gv2, element_size 16, index_type i32
;;
;;                                 block0(v0: i64):
;;                                     v13 -> v0
;;                                     v14 -> v0
;; @0051                               v2 = iconst.i32 0
;; @0053                               v5 = load.i32 notrap aligned readonly v13
;; @0053                               v6 = icmp uge v2, v5  ; v2 = 0
;; @0053                               brif v6, block2, block3
;;
;;                                 block2 cold:
;; @0053                               trap table_oob
;;
;;                                 block3:
;; @0053                               v7 = uextend.i64 v2  ; v2 = 0
;; @0053                               v8 = load.i64 notrap aligned readonly v14
;;                                     v15 = iconst.i64 4
;; @0053                               v9 = ishl v7, v15  ; v15 = 4
;; @0053                               v10 = iadd v8, v9
;; @0053                               v11 = icmp.i32 uge v2, v5  ; v2 = 0
;; @0053                               v12 = select_spectre_guard v11, v8, v10
;;                                     v3 -> v12
;; @0053                               v4 = load.r64 notrap aligned table v3
;;                                     v1 -> v4
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v1
;; }
;;
;; function u0:1(i32, i64 vmctx) -> r64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;     gv2 = load.i32 notrap aligned readonly gv0
;;     table0 = dynamic gv1, min 0, bound gv2, element_size 16, index_type i32
;;
;;                                 block0(v0: i32, v1: i64):
;;                                     v13 -> v1
;;                                     v14 -> v1
;; @005a                               v5 = load.i32 notrap aligned readonly v13
;; @005a                               v6 = icmp uge v0, v5
;; @005a                               brif v6, block2, block3
;;
;;                                 block2 cold:
;; @005a                               trap table_oob
;;
;;                                 block3:
;; @005a                               v7 = uextend.i64 v0
;; @005a                               v8 = load.i64 notrap aligned readonly v14
;;                                     v15 = iconst.i64 4
;; @005a                               v9 = ishl v7, v15  ; v15 = 4
;; @005a                               v10 = iadd v8, v9
;; @005a                               v11 = icmp.i32 uge v0, v5
;; @005a                               v12 = select_spectre_guard v11, v8, v10
;;                                     v3 -> v12
;; @005a                               v4 = load.r64 notrap aligned table v3
;;                                     v2 -> v4
;; @005c                               jump block1
;;
;;                                 block1:
;; @005c                               return v2
;; }
