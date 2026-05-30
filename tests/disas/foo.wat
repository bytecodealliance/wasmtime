;;! target = "x86_64"
;;! test = "optimize"


(module
  (import "" "" (table 1 funcref))
  (memory 1)
  (func (export "i32.load") (param i32 i32) (result i32 i32)
    local.get 0
    (i32.load (local.get 1))
    call_indirect (param i32) (result i32)
    local.get 0
    (i32.load (local.get 1))
    call_indirect (param i32) (result i32)
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32, i32 tail {
;;     region0 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     region1 = 1073741824 "ImportedTable"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+72
;;     gv7 = load.i64 notrap aligned gv6
;;     gv8 = load.i64 notrap aligned gv6+8
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0040                               v7 = load.i64 notrap aligned readonly can_move v0+56
;; @0040                               v6 = uextend.i64 v3
;; @0040                               v8 = iadd v7, v6
;; @0040                               v9 = load.i32 little region0 v8
;; @0043                               v75 = load.i64 notrap aligned readonly can_move v0+72
;; @0043                               v10 = load.i64 notrap aligned v75+8
;; @0043                               v14 = load.i64 notrap aligned v75
;; @0043                               v11 = ireduce.i32 v10
;; @0043                               v12 = icmp uge v9, v11
;; @0043                               v17 = iconst.i64 0
;; @0043                               v13 = uextend.i64 v9
;;                                     v72 = iconst.i64 3
;; @0043                               v15 = ishl v13, v72  ; v72 = 3
;; @0043                               v16 = iadd v14, v15
;; @0043                               v18 = select_spectre_guard v12, v17, v16  ; v17 = 0
;; @0043                               v19 = load.i64 user6 aligned region1 v18
;; @0043                               v20 = iconst.i64 -2
;; @0043                               v21 = band v19, v20  ; v20 = -2
;; @0043                               brif v19, block3(v21), block2
;;
;;                                 block2 cold:
;; @0043                               v23 = iconst.i32 0
;; @0043                               v26 = call fn0(v0, v23, v13)  ; v23 = 0
;; @0043                               jump block3(v26)
;;
;;                                 block3(v22: i64):
;; @0043                               v30 = load.i32 user7 aligned readonly v22+16
;; @0043                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @0043                               v29 = load.i32 notrap aligned readonly can_move v28+4
;; @0043                               v31 = icmp eq v30, v29
;; @0043                               trapz v31, user8
;; @0043                               v32 = load.i64 notrap aligned readonly v22+8
;; @0043                               v33 = load.i64 notrap aligned readonly v22+24
;; @0043                               v34 = call_indirect sig0, v32(v33, v0, v2)
;; @004a                               v40 = load.i32 little region0 v8
;; @004d                               v41 = load.i64 notrap aligned v75+8
;; @004d                               v45 = load.i64 notrap aligned v75
;; @004d                               v42 = ireduce.i32 v41
;; @004d                               v43 = icmp uge v40, v42
;; @004d                               v44 = uextend.i64 v40
;;                                     v78 = iconst.i64 3
;;                                     v79 = ishl v44, v78  ; v78 = 3
;; @004d                               v47 = iadd v45, v79
;;                                     v80 = iconst.i64 0
;;                                     v81 = select_spectre_guard v43, v80, v47  ; v80 = 0
;; @004d                               v50 = load.i64 user6 aligned region1 v81
;;                                     v82 = iconst.i64 -2
;;                                     v83 = band v50, v82  ; v82 = -2
;; @004d                               brif v50, block5(v83), block4
;;
;;                                 block4 cold:
;;                                     v84 = iconst.i32 0
;; @004d                               v57 = call fn0(v0, v84, v44)  ; v84 = 0
;; @004d                               jump block5(v57)
;;
;;                                 block5(v53: i64):
;; @004d                               v61 = load.i32 user7 aligned readonly v53+16
;; @004d                               v62 = icmp eq v61, v29
;; @004d                               trapz v62, user8
;; @004d                               v63 = load.i64 notrap aligned readonly v53+8
;; @004d                               v64 = load.i64 notrap aligned readonly v53+24
;; @004d                               v65 = call_indirect sig0, v63(v64, v0, v2)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v34, v65
;; }
