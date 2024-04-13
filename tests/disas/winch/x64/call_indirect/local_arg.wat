;;! target="x86_64"

(module
    (type $param-i32 (func (param i32)))

    (func $param-i32 (type $param-i32))
    (func (export "")
        (local i32)
        local.get 0
        (call_indirect (type $param-i32) (i32.const 0))
    )

    (table funcref
      (elem
        $param-i32)
    )
)

;; function u0:0(i64 vmctx, i64, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0032                               jump block1
;;
;;                                 block1:
;; @0032                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly gv3+88
;;     sig0 = (i64 vmctx, i64, i32) fast
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:9 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0035                               v2 = iconst.i32 0
;; @0039                               v3 = iconst.i32 0
;; @003b                               v4 = iconst.i32 1
;; @003b                               v5 = icmp uge v3, v4  ; v3 = 0, v4 = 1
;; @003b                               v6 = uextend.i64 v3  ; v3 = 0
;; @003b                               v7 = global_value.i64 gv4
;; @003b                               v8 = ishl_imm v6, 3
;; @003b                               v9 = iadd v7, v8
;; @003b                               v10 = iconst.i64 0
;; @003b                               v11 = select_spectre_guard v5, v10, v9  ; v10 = 0
;; @003b                               v12 = load.i64 table_oob aligned table v11
;; @003b                               v13 = band_imm v12, -2
;; @003b                               brif v12, block3(v13), block2
;;
;;                                 block2 cold:
;; @003b                               v15 = iconst.i32 0
;; @003b                               v16 = global_value.i64 gv3
;; @003b                               v17 = call fn0(v16, v15, v3)  ; v15 = 0, v3 = 0
;; @003b                               jump block3(v17)
;;
;;                                 block3(v14: i64):
;; @003b                               v18 = global_value.i64 gv3
;; @003b                               v19 = load.i64 notrap aligned readonly v18+80
;; @003b                               v20 = load.i32 notrap aligned readonly v19
;; @003b                               v21 = load.i32 icall_null aligned readonly v14+24
;; @003b                               v22 = icmp eq v21, v20
;; @003b                               trapz v22, bad_sig
;; @003b                               v23 = load.i64 notrap aligned readonly v14+16
;; @003b                               v24 = load.i64 notrap aligned readonly v14+32
;; @003b                               call_indirect sig0, v23(v24, v0, v2)  ; v2 = 0
;; @003e                               jump block1
;;
;;                                 block1:
;; @003e                               return
;; }
