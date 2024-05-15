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
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v3 = iconst.i32 2
;; @0031                               v4 = icmp uge v2, v3  ; v3 = 2
;; @0031                               v9 = iconst.i64 0
;; @0031                               v6 = load.i64 notrap aligned readonly v0+88
;; @0031                               v5 = uextend.i64 v2
;;                                     v25 = iconst.i64 3
;; @0031                               v7 = ishl v5, v25  ; v25 = 3
;; @0031                               v8 = iadd v6, v7
;; @0031                               v10 = select_spectre_guard v4, v9, v8  ; v9 = 0
;; @0031                               v11 = load.i64 table_oob aligned table v10
;;                                     v26 = iconst.i64 -2
;; @0031                               v12 = band v11, v26  ; v26 = -2
;; @0031                               brif v11, block3(v12), block2
;;
;;                                 block2 cold:
;; @0031                               v14 = iconst.i32 0
;; @0031                               v16 = call fn0(v0, v14, v2)  ; v14 = 0
;; @0031                               jump block3(v16)
;;
;;                                 block3(v13: i64):
;; @0031                               v20 = load.i32 icall_null aligned readonly v13+16
;; @0031                               v18 = load.i64 notrap aligned readonly v0+80
;; @0031                               v19 = load.i32 notrap aligned readonly v18
;; @0031                               v21 = icmp eq v20, v19
;; @0031                               brif v21, block5, block4
;;
;;                                 block4 cold:
;; @0031                               trap bad_sig
;;
;;                                 block5:
;; @0031                               v22 = load.i64 notrap aligned readonly v13+8
;; @0031                               v23 = load.i64 notrap aligned readonly v13+24
;; @0031                               call_indirect sig0, v22(v23, v0)
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }
