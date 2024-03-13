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
;;
;;                                 block0(v0: r64, v1: i64):
;;                                     v11 -> v1
;;                                     v12 -> v1
;; @0051                               v2 = iconst.i32 0
;; @0055                               v3 = load.i32 notrap aligned readonly v11
;; @0055                               v4 = icmp uge v2, v3  ; v2 = 0
;; @0055                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @0055                               trap table_oob
;;
;;                                 block3:
;; @0055                               v5 = uextend.i64 v2  ; v2 = 0
;; @0055                               v6 = load.i64 notrap aligned readonly v12
;;                                     v13 = iconst.i64 4
;; @0055                               v7 = ishl v5, v13  ; v13 = 4
;; @0055                               v8 = iadd v6, v7
;; @0055                               v9 = icmp.i32 uge v2, v3  ; v2 = 0
;; @0055                               v10 = select_spectre_guard v9, v6, v8
;; @0055                               store.r64 notrap aligned table v0, v10
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
;;
;;                                 block0(v0: i32, v1: r64, v2: i64):
;;                                     v11 -> v2
;;                                     v12 -> v2
;; @005e                               v3 = load.i32 notrap aligned readonly v11
;; @005e                               v4 = icmp uge v0, v3
;; @005e                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @005e                               trap table_oob
;;
;;                                 block3:
;; @005e                               v5 = uextend.i64 v0
;; @005e                               v6 = load.i64 notrap aligned readonly v12
;;                                     v13 = iconst.i64 4
;; @005e                               v7 = ishl v5, v13  ; v13 = 4
;; @005e                               v8 = iadd v6, v7
;; @005e                               v9 = icmp.i32 uge v0, v3
;; @005e                               v10 = select_spectre_guard v9, v6, v8
;; @005e                               store.r64 notrap aligned table v1, v10
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
