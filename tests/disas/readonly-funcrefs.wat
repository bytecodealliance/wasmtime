;;! target = "x86_64"
;;! test = "optimize"

;; Current WebAssembly toolchains represent source-language function pointers
;; by constructing a single WebAssembly funcref table, initializing it with an
;; element section, and never writing to it again. This test tracks what code
;; we generate for that pattern.

;; Motivated by https://github.com/bytecodealliance/wasmtime/issues/8195

(module
  (type $fn (func))
  (table $fnptrs 2 2 funcref)
  (func $callee)
  (func $caller (param i32)
        local.get 0
        call_indirect $fnptrs (type $fn))
  (elem $fnptrs (i32.const 1) func $callee)
)

;; function u0:0(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002c                               jump block1
;;
;;                                 block1:
;; @002c                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64) fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v15 -> v0
;;                                     v19 -> v0
;;                                     v26 -> v0
;; @0031                               v6 = load.i64 notrap aligned v0+72
;; @0031                               v3 = iconst.i32 2
;; @0031                               v4 = icmp uge v2, v3  ; v3 = 2
;; @0031                               v9 = iconst.i64 0
;; @0031                               v5 = uextend.i64 v2
;;                                     v27 = iconst.i64 3
;; @0031                               v7 = ishl v5, v27  ; v27 = 3
;; @0031                               v8 = iadd v6, v7
;; @0031                               v10 = select_spectre_guard v4, v9, v8  ; v9 = 0
;; @0031                               v11 = load.i64 table_oob aligned table v10
;;                                     v28 = iconst.i64 -2
;; @0031                               v12 = band v11, v28  ; v28 = -2
;; @0031                               brif v11, block3(v12), block2
;;
;;                                 block2 cold:
;; @0031                               v16 = load.i64 notrap aligned readonly v0+56
;; @0031                               v17 = load.i64 notrap aligned readonly v16+72
;; @0031                               v14 = iconst.i32 0
;; @0031                               v18 = call_indirect sig1, v17(v0, v14, v2)  ; v14 = 0
;; @0031                               jump block3(v18)
;;
;;                                 block3(v13: i64):
;; @0031                               v22 = load.i32 icall_null aligned readonly v13+24
;; @0031                               v20 = load.i64 notrap aligned readonly v0+64
;; @0031                               v21 = load.i32 notrap aligned readonly v20
;; @0031                               v23 = icmp eq v22, v21
;; @0031                               brif v23, block5, block4
;;
;;                                 block4 cold:
;; @0031                               trap bad_sig
;;
;;                                 block5:
;; @0031                               v24 = load.i64 notrap aligned readonly v13+16
;; @0031                               v25 = load.i64 notrap aligned readonly v13+32
;; @0031                               call_indirect sig0, v24(v25, v0)
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
