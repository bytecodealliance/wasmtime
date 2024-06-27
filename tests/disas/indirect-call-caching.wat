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
;; @0050                               v5 = band_imm v2, 1023
;; @0050                               v6 = iadd_imm v5, 0
;; @0050                               v7 = iadd_imm v4, 240
;; @0050                               v8 = iadd_imm v4, 4336
;; @0050                               v9 = iadd_imm v4, 8432
;; @0050                               v10 = ishl_imm v6, 2
;; @0050                               v11 = ishl_imm v6, 3
;; @0050                               v12 = iadd v7, v10
;; @0050                               v13 = iadd v8, v10
;; @0050                               v14 = iadd v9, v11
;; @0050                               v15 = load.i32 notrap aligned v12
;; @0050                               v16 = icmp eq v15, v2
;; @0050                               brif v16, block2, block4
;;
;;                                 block2:
;; @0050                               v17 = load.i32 notrap aligned v13
;; @0050                               v18 = load.i64 notrap aligned v14
;; @0050                               v19 = icmp_imm eq v17, 0
;; @0050                               brif v19, block3, block4
;;
;;                                 block3:
;; @0050                               v20 = icmp_imm.i64 ne v18, 0
;; @0050                               brif v20, block5(v18, v4), block4
;;
;;                                 block4 cold:
;; @0050                               v21 = iconst.i32 10
;; @0050                               v22 = icmp.i32 uge v2, v21  ; v21 = 10
;; @0050                               v23 = uextend.i64 v2
;; @0050                               v24 = global_value.i64 gv4
;; @0050                               v25 = ishl_imm v23, 3
;; @0050                               v26 = iadd v24, v25
;; @0050                               v27 = iconst.i64 0
;; @0050                               v28 = select_spectre_guard v22, v27, v26  ; v27 = 0
;; @0050                               v29 = load.i64 table_oob aligned table v28
;; @0050                               v30 = band_imm v29, -2
;; @0050                               brif v29, block8(v30), block7
;;
;;                                 block6 cold:
;; @0050                               store.i32 notrap aligned v2, v12
;; @0050                               v43 = iconst.i32 0
;; @0050                               store notrap aligned v43, v13  ; v43 = 0
;; @0050                               store.i64 notrap aligned v40, v14
;; @0050                               jump block5(v40, v41)
;;
;;                                 block5(v44: i64, v45: i64):
;; @0050                               v46 = call_indirect sig0, v44(v45, v0)
;; @0053                               jump block1(v46)
;;
;;                                 block7 cold:
;; @0050                               v32 = iconst.i32 0
;; @0050                               v33 = global_value.i64 gv3
;; @0050                               v34 = call fn0(v33, v32, v2)  ; v32 = 0
;; @0050                               jump block8(v34)
;;
;;                                 block8(v31: i64) cold:
;; @0050                               v35 = global_value.i64 gv3
;; @0050                               v36 = load.i64 notrap aligned readonly v35+80
;; @0050                               v37 = load.i32 notrap aligned readonly v36
;; @0050                               v38 = load.i32 icall_null aligned readonly v31+16
;; @0050                               v39 = icmp eq v38, v37
;; @0050                               trapz v39, bad_sig
;; @0050                               v40 = load.i64 notrap aligned readonly v31+8
;; @0050                               v41 = load.i64 notrap aligned readonly v31+24
;; @0050                               v42 = icmp eq v41, v4
;; @0050                               brif v42, block6, block5(v40, v41)
;;
;;                                 block1(v3: i32):
;; @0053                               return v3
;; }
