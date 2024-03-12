;;! target = "x86_64"
;;! optimize = true

;; Test basic code generation for table WebAssembly instructions.
;; Use optimization but with opt-level "none" to legalize away table_addr instructions.

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

;; function u0:0(r64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;     gv2 = load.i32 notrap aligned readonly gv0
;;     table0 = dynamic gv1, min 0, bound gv2, element_size 16, index_type i32
;;
;;                                 block0(v0: r64, v1: i64):
;;                                     v12 -> v1
;;                                     v13 -> v1
;; @0051                               v2 = iconst.i32 0
;; @0055                               v4 = load.i32 notrap aligned readonly v12
;; @0055                               v5 = icmp uge v2, v4  ; v2 = 0
;; @0055                               brif v5, block2, block3
;;
;;                                 block2 cold:
;; @0055                               trap table_oob
;;
;;                                 block3:
;; @0055                               v6 = uextend.i64 v2  ; v2 = 0
;; @0055                               v7 = load.i64 notrap aligned readonly v13
;;                                     v14 = iconst.i64 4
;; @0055                               v8 = ishl v6, v14  ; v14 = 4
;; @0055                               v9 = iadd v7, v8
;; @0055                               v10 = icmp.i32 uge v2, v4  ; v2 = 0
;; @0055                               v11 = select_spectre_guard v10, v7, v9
;;                                     v3 -> v11
;; @0055                               store.r64 notrap aligned table v0, v3
;; @0057                               jump block1
;;
;;                                 block1:
;; @0057                               return
;; }
;;
;; function u0:1(i32, r64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;     gv2 = load.i32 notrap aligned readonly gv0
;;     table0 = dynamic gv1, min 0, bound gv2, element_size 16, index_type i32
;;
;;                                 block0(v0: i32, v1: r64, v2: i64):
;;                                     v12 -> v2
;;                                     v13 -> v2
;; @005e                               v4 = load.i32 notrap aligned readonly v12
;; @005e                               v5 = icmp uge v0, v4
;; @005e                               brif v5, block2, block3
;;
;;                                 block2 cold:
;; @005e                               trap table_oob
;;
;;                                 block3:
;; @005e                               v6 = uextend.i64 v0
;; @005e                               v7 = load.i64 notrap aligned readonly v13
;;                                     v14 = iconst.i64 4
;; @005e                               v8 = ishl v6, v14  ; v14 = 4
;; @005e                               v9 = iadd v7, v8
;; @005e                               v10 = icmp.i32 uge v0, v4
;; @005e                               v11 = select_spectre_guard v10, v7, v9
;;                                     v3 -> v11
;; @005e                               store.r64 notrap aligned table v1, v3
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
