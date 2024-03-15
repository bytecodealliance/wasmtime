;;! target = "x86_64"

(module
  (type $ft (func (param v128) (result v128)))
  (func $foo (export "foo") (param i32) (param v128) (result v128)
    (call_indirect (type $ft) (local.get 1) (local.get 0))
  )
  (table (;0;) 23 23 funcref)
)

;; function u0:0(i64 vmctx, i64, i32, i8x16) -> i8x16 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+72
;;     sig0 = (i64 vmctx, i64, i8x16) -> i8x16 fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     sig2 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig3 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i8x16):
;; @0033                               v5 = iconst.i32 23
;; @0033                               v6 = icmp uge v2, v5  ; v5 = 23
;; @0033                               trapnz v6, table_oob
;; @0033                               v7 = uextend.i64 v2
;; @0033                               v8 = global_value.i64 gv4
;; @0033                               v9 = ishl_imm v7, 3
;; @0033                               v10 = iadd v8, v9
;; @0033                               v11 = icmp uge v2, v5  ; v5 = 23
;; @0033                               v12 = select_spectre_guard v11, v8, v10
;; @0033                               v13 = load.i64 notrap aligned table v12
;; @0033                               v14 = band_imm v13, -2
;; @0033                               brif v13, block3(v14), block2
;;
;;                                 block2 cold:
;; @0033                               v16 = iconst.i32 0
;; @0033                               v17 = global_value.i64 gv3
;; @0033                               v18 = load.i64 notrap aligned readonly v17+56
;; @0033                               v19 = load.i64 notrap aligned readonly v18+72
;; @0033                               v20 = call_indirect sig1, v19(v17, v16, v2)  ; v16 = 0
;; @0033                               jump block3(v20)
;;
;;                                 block3(v15: i64):
;; @0033                               trapz v15, icall_null
;; @0033                               v21 = global_value.i64 gv3
;; @0033                               v22 = load.i64 notrap aligned readonly v21+64
;; @0033                               v23 = load.i32 notrap aligned readonly v22
;; @0033                               v24 = load.i32 notrap aligned readonly v15+24
;; @0033                               v25 = icmp eq v24, v23
;; @0033                               trapz v25, bad_sig
;; @0033                               v26 = load.i64 notrap aligned readonly v15+16
;; @0033                               v27 = load.i64 notrap aligned readonly v15+32
;; @0033                               v28 = call_indirect sig0, v26(v27, v0, v3)
;; @0036                               jump block1(v28)
;;
;;                                 block1(v4: i8x16):
;; @0036                               return v4
;; }
