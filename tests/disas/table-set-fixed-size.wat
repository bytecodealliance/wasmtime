;;! target = "x86_64"
;;! optimize = true

;; Test basic code generation for table WebAssembly instructions on
;; non-resizeable tables. Use optimization but with opt-level "none" to
;; legalize away macro instructions.

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

;; function u0:0(r64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: r64, v1: i64):
;;                                     v11 -> v1
;; @0052                               v2 = iconst.i32 0
;; @0056                               v3 = iconst.i32 7
;; @0056                               v4 = icmp uge v2, v3  ; v2 = 0, v3 = 7
;; @0056                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @0056                               trap table_oob
;;
;;                                 block3:
;; @0056                               v5 = uextend.i64 v2  ; v2 = 0
;; @0056                               v6 = load.i64 notrap aligned readonly v11
;;                                     v12 = iconst.i64 4
;; @0056                               v7 = ishl v5, v12  ; v12 = 4
;; @0056                               v8 = iadd v6, v7
;; @0056                               v9 = icmp.i32 uge v2, v3  ; v2 = 0, v3 = 7
;; @0056                               v10 = select_spectre_guard v9, v6, v8
;; @0056                               store.r64 notrap aligned table v0, v10
;; @0058                               jump block1
;;
;;                                 block1:
;; @0058                               return
;; }
;;
;; function u0:1(i32, r64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: r64, v2: i64):
;;                                     v11 -> v2
;; @005f                               v3 = iconst.i32 7
;; @005f                               v4 = icmp uge v0, v3  ; v3 = 7
;; @005f                               brif v4, block2, block3
;;
;;                                 block2 cold:
;; @005f                               trap table_oob
;;
;;                                 block3:
;; @005f                               v5 = uextend.i64 v0
;; @005f                               v6 = load.i64 notrap aligned readonly v11
;;                                     v12 = iconst.i64 4
;; @005f                               v7 = ishl v5, v12  ; v12 = 4
;; @005f                               v8 = iadd v6, v7
;; @005f                               v9 = icmp.i32 uge v0, v3  ; v3 = 7
;; @005f                               v10 = select_spectre_guard v9, v6, v8
;; @005f                               store.r64 notrap aligned table v1, v10
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return
;; }
