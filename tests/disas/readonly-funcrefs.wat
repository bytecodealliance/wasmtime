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

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @002c                               jump block1
;;
;;                                 block1:
;; @002c                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v3 = iconst.i32 2
;; @0031                               v4 = icmp uge v2, v3  ; v3 = 2
;; @0031                               v9 = iconst.i64 0
;; @0031                               v6 = load.i64 notrap aligned readonly can_move v0+48
;; @0031                               v5 = uextend.i64 v2
;;                                     v21 = iconst.i64 3
;; @0031                               v7 = ishl v5, v21  ; v21 = 3
;; @0031                               v8 = iadd v6, v7
;; @0031                               v10 = select_spectre_guard v4, v9, v8  ; v9 = 0
;; @0031                               v11 = load.i64 user6 aligned table v10
;;                                     v20 = iconst.i64 -2
;; @0031                               v12 = band v11, v20  ; v20 = -2
;; @0031                               brif v11, block3(v12), block2
;;
;;                                 block2 cold:
;; @0031                               v14 = iconst.i32 0
;; @0031                               v17 = call fn0(v0, v14, v5)  ; v14 = 0
;; @0031                               jump block3(v17)
;;
;;                                 block3(v13: i64):
;; @0031                               v18 = load.i64 user7 aligned readonly v13+8
;; @0031                               v19 = load.i64 notrap aligned readonly v13+24
;; @0031                               call_indirect sig0, v18(v19, v0)
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
