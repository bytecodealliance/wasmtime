;;! target = "x86_64"

(module
  (type $ft (func (param v128) (result v128)))
  (func $foo (export "foo") (param i32) (param v128) (result v128)
    (call_indirect (type $ft) (local.get 1) (local.get 0))
  )
  (table (;0;) 23 23 funcref)
)

;; function u0:0(i64 vmctx, i64, i32, i8x16) -> i8x16 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2684354560 "VMTableDefinition+0x0"
;;     region3 = 1342177280 "DefinedTable(StaticModuleIndex(0), DefinedTableIndex(0))"
;;     region4 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i8x16) -> i8x16 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i8x16):
;; @0033                               v4 = iconst.i32 23
;; @0033                               v5 = icmp uge v2, v4  ; v4 = 23
;; @0033                               v6 = uextend.i64 v2
;; @0033                               v7 = load.i64 notrap aligned readonly can_move region2 v0+48
;; @0033                               v8 = iconst.i64 3
;; @0033                               v9 = ishl v6, v8  ; v8 = 3
;; @0033                               v10 = iadd v7, v9
;; @0033                               v11 = iconst.i64 0
;; @0033                               v12 = select_spectre_guard v5, v11, v10  ; v11 = 0
;; @0033                               v13 = load.i64 user6 aligned region3 v12
;; @0033                               v14 = iconst.i64 -2
;; @0033                               v15 = band v13, v14  ; v14 = -2
;; @0033                               brif v13, block3(v15), block2
;;
;;                                 block2 cold:
;; @0033                               v17 = iconst.i32 0
;; @0033                               v18 = uextend.i64 v2
;; @0033                               v19 = call fn0(v0, v17, v18)  ; v17 = 0
;; @0033                               jump block3(v19)
;;
;;                                 block3(v16: i64):
;; @0033                               v20 = load.i64 notrap aligned readonly can_move region4 v0+40
;; @0033                               v21 = load.i32 notrap aligned readonly can_move v20
;; @0033                               v22 = load.i32 user7 aligned readonly v16+16
;; @0033                               v23 = icmp eq v22, v21
;; @0033                               v24 = uextend.i32 v23
;; @0033                               trapz v24, user8
;; @0033                               v25 = load.i64 notrap aligned readonly v16+8
;; @0033                               v26 = load.i64 notrap aligned readonly v16+24
;; @0033                               v27 = call_indirect sig0, v25(v26, v0, v3)
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v27
;; }
