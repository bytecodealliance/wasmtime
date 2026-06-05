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
;;     region1 = 1073741824 "PublicTable"
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
;; @0043                               v77 = load.i64 notrap aligned readonly can_move v0+72
;; @0043                               v10 = load.i64 notrap aligned v77+8
;; @0043                               v14 = load.i64 notrap aligned v77
;; @0043                               v11 = ireduce.i32 v10
;; @0043                               v12 = icmp uge v9, v11
;; @0043                               v18 = iconst.i64 0
;; @0043                               v13 = uextend.i64 v9
;; @0043                               v15 = iconst.i64 3
;; @0043                               v16 = ishl v13, v15  ; v15 = 3
;; @0043                               v17 = iadd v14, v16
;; @0043                               v19 = select_spectre_guard v12, v18, v17  ; v18 = 0
;; @0043                               v20 = load.i64 user6 aligned region1 v19
;; @0043                               v21 = iconst.i64 -2
;; @0043                               v22 = band v20, v21  ; v21 = -2
;; @0043                               brif v20, block3(v22), block2
;;
;;                                 block2 cold:
;; @0043                               v24 = iconst.i32 0
;; @0043                               v27 = call fn0(v0, v24, v13)  ; v24 = 0
;; @0043                               jump block3(v27)
;;
;;                                 block3(v23: i64):
;; @0043                               v31 = load.i32 user7 aligned readonly v23+16
;; @0043                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @0043                               v30 = load.i32 notrap aligned readonly can_move v29+4
;; @0043                               v32 = icmp eq v31, v30
;; @0043                               trapz v32, user8
;; @0043                               v34 = load.i64 notrap aligned readonly v23+8
;; @0043                               v35 = load.i64 notrap aligned readonly v23+24
;; @0043                               v36 = call_indirect sig0, v34(v35, v0, v2)
;; @004a                               v42 = load.i32 little region0 v8
;; @004d                               v43 = load.i64 notrap aligned v77+8
;; @004d                               v47 = load.i64 notrap aligned v77
;; @004d                               v44 = ireduce.i32 v43
;; @004d                               v45 = icmp uge v42, v44
;; @004d                               v46 = uextend.i64 v42
;;                                     v80 = iconst.i64 3
;;                                     v81 = ishl v46, v80  ; v80 = 3
;; @004d                               v50 = iadd v47, v81
;;                                     v82 = iconst.i64 0
;;                                     v83 = select_spectre_guard v45, v82, v50  ; v82 = 0
;; @004d                               v53 = load.i64 user6 aligned region1 v83
;;                                     v84 = iconst.i64 -2
;;                                     v85 = band v53, v84  ; v84 = -2
;; @004d                               brif v53, block5(v85), block4
;;
;;                                 block4 cold:
;;                                     v86 = iconst.i32 0
;; @004d                               v60 = call fn0(v0, v86, v46)  ; v86 = 0
;; @004d                               jump block5(v60)
;;
;;                                 block5(v56: i64):
;; @004d                               v64 = load.i32 user7 aligned readonly v56+16
;; @004d                               v65 = icmp eq v64, v30
;; @004d                               trapz v65, user8
;; @004d                               v67 = load.i64 notrap aligned readonly v56+8
;; @004d                               v68 = load.i64 notrap aligned readonly v56+24
;; @004d                               v69 = call_indirect sig0, v67(v68, v0, v2)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v36, v69
;; }
