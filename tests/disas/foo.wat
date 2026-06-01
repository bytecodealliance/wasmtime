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
;;                                     v71 = iconst.i64 -2
;; @0043                               v20 = band v19, v71  ; v71 = -2
;; @0043                               brif v19, block3(v20), block2
;;
;;                                 block2 cold:
;; @0043                               v22 = iconst.i32 0
;; @0043                               v25 = call fn0(v0, v22, v13)  ; v22 = 0
;; @0043                               jump block3(v25)
;;
;;                                 block3(v21: i64):
;; @0043                               v29 = load.i32 user7 aligned readonly v21+16
;; @0043                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @0043                               v28 = load.i32 notrap aligned readonly can_move v27+4
;; @0043                               v30 = icmp eq v29, v28
;; @0043                               trapz v30, user8
;; @0043                               v31 = load.i64 notrap aligned readonly v21+8
;; @0043                               v32 = load.i64 notrap aligned readonly v21+24
;; @0043                               v33 = call_indirect sig0, v31(v32, v0, v2)
;; @004a                               v39 = load.i32 little region0 v8
;; @004d                               v40 = load.i64 notrap aligned v75+8
;; @004d                               v44 = load.i64 notrap aligned v75
;; @004d                               v41 = ireduce.i32 v40
;; @004d                               v42 = icmp uge v39, v41
;; @004d                               v43 = uextend.i64 v39
;;                                     v78 = iconst.i64 3
;;                                     v79 = ishl v43, v78  ; v78 = 3
;; @004d                               v46 = iadd v44, v79
;;                                     v80 = iconst.i64 0
;;                                     v81 = select_spectre_guard v42, v80, v46  ; v80 = 0
;; @004d                               v49 = load.i64 user6 aligned region1 v81
;;                                     v82 = iconst.i64 -2
;;                                     v83 = band v49, v82  ; v82 = -2
;; @004d                               brif v49, block5(v83), block4
;;
;;                                 block4 cold:
;;                                     v84 = iconst.i32 0
;; @004d                               v55 = call fn0(v0, v84, v43)  ; v84 = 0
;; @004d                               jump block5(v55)
;;
;;                                 block5(v51: i64):
;; @004d                               v59 = load.i32 user7 aligned readonly v51+16
;; @004d                               v60 = icmp eq v59, v28
;; @004d                               trapz v60, user8
;; @004d                               v61 = load.i64 notrap aligned readonly v51+8
;; @004d                               v62 = load.i64 notrap aligned readonly v51+24
;; @004d                               v63 = call_indirect sig0, v61(v62, v0, v2)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v33, v63
;; }
