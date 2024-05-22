;;! target = "x86_64"
;;! flags = [ "-Ocache-call-indirects=y" ]

;; This test checks that we get the indirect-call caching optimization
;; where it should be applicable (immutable table, null 0-index).
;;
;; The key bit in the expectation below is the cached-index load (v6),
;; compare (v7), branch, fastpath in block2/block4.

(module
 (table 10 10 funcref)

 (func $f1 (result i32) i32.const 1)
 (func $f2 (result i32) i32.const 2)
 (func $f3 (result i32) i32.const 3)

 (func (export "call_it") (param i32) (result i32)
  local.get 0
  call_indirect (result i32))

 (elem (i32.const 1) func $f1 $f2 $f3))
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003f                               v3 = iconst.i32 1
;; @0041                               jump block1(v3)  ; v3 = 1
;;
;;                                 block1(v2: i32):
;; @0041                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v3 = iconst.i32 2
;; @0046                               jump block1(v3)  ; v3 = 2
;;
;;                                 block1(v2: i32):
;; @0046                               return v2
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0049                               v3 = iconst.i32 3
;; @004b                               jump block1(v3)  ; v3 = 3
;;
;;                                 block1(v2: i32):
;; @004b                               return v2
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0050                               v4 = global_value.i64 gv3
;; @0050                               v5 = iadd_imm v4, 240
;; @0050                               v6 = load.i32 notrap aligned v5+8
;; @0050                               v7 = load.i64 notrap aligned v5
;; @0050                               v8 = icmp eq v6, v2
;; @0050                               brif v8, block3(v7, v4), block2
;;
;;                                 block2 cold:
;; @0050                               v9 = iconst.i32 10
;; @0050                               v10 = icmp.i32 uge v2, v9  ; v9 = 10
;; @0050                               v11 = uextend.i64 v2
;; @0050                               v12 = global_value.i64 gv4
;; @0050                               v13 = ishl_imm v11, 3
;; @0050                               v14 = iadd v12, v13
;; @0050                               v15 = iconst.i64 0
;; @0050                               v16 = select_spectre_guard v10, v15, v14  ; v15 = 0
;; @0050                               v17 = load.i64 table_oob aligned table v16
;; @0050                               v18 = band_imm v17, -2
;; @0050                               brif v17, block6(v18), block5
;;
;;                                 block4 cold:
;; @0050                               store.i32 notrap aligned v2, v5+8
;; @0050                               store.i64 notrap aligned v28, v5
;; @0050                               jump block3(v28, v29)
;;
;;                                 block3(v31: i64, v32: i64):
;; @0050                               v33 = call_indirect sig0, v31(v32, v0)
;; @0053                               jump block1(v33)
;;
;;                                 block5 cold:
;; @0050                               v20 = iconst.i32 0
;; @0050                               v21 = global_value.i64 gv3
;; @0050                               v22 = call fn0(v21, v20, v2)  ; v20 = 0
;; @0050                               jump block6(v22)
;;
;;                                 block6(v19: i64) cold:
;; @0050                               v23 = global_value.i64 gv3
;; @0050                               v24 = load.i64 notrap aligned readonly v23+80
;; @0050                               v25 = load.i32 notrap aligned readonly v24
;; @0050                               v26 = load.i32 icall_null aligned readonly v19+16
;; @0050                               v27 = icmp eq v26, v25
;; @0050                               trapz v27, bad_sig
;; @0050                               v28 = load.i64 notrap aligned readonly v19+8
;; @0050                               v29 = load.i64 notrap aligned readonly v19+24
;; @0050                               v30 = icmp eq v29, v4
;; @0050                               brif v30, block4, block3(v28, v29)
;;
;;                                 block1(v3: i32):
;; @0053                               return v3
;; }
