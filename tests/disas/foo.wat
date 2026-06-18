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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     region5 = 72 "VMContext+0x48"
;;     region6 = 2684354560 "VMTableDefinition+0x0"
;;     region7 = 2684354568 "VMTableDefinition+0x8"
;;     region8 = 1073741824 "PublicTable"
;;     region9 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i32, i64) -> i64 tail
;;     fn0 = colocated u805306368:7 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0040                               v7 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0040                               v6 = uextend.i64 v3
;; @0040                               v8 = iadd v7, v6
;; @0040                               v9 = load.i32 little region4 v8
;; @0043                               v10 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @0043                               v11 = load.i64 notrap aligned region7 v10+8
;; @0043                               v16 = load.i64 notrap aligned region6 v10
;; @0043                               v12 = ireduce.i32 v11
;; @0043                               v13 = icmp uge v9, v12
;; @0043                               v20 = iconst.i64 0
;; @0043                               v14 = uextend.i64 v9
;; @0043                               v17 = iconst.i64 3
;; @0043                               v18 = ishl v14, v17  ; v17 = 3
;; @0043                               v19 = iadd v16, v18
;; @0043                               v21 = select_spectre_guard v13, v20, v19  ; v20 = 0
;; @0043                               v22 = load.i64 user6 aligned region8 v21
;; @0043                               v23 = iconst.i64 -2
;; @0043                               v24 = band v22, v23  ; v23 = -2
;; @0043                               brif v22, block3(v24), block2
;;
;;                                 block2 cold:
;; @0043                               v26 = iconst.i32 0
;; @0043                               v28 = call fn0(v0, v26, v14)  ; v26 = 0
;; @0043                               jump block3(v28)
;;
;;                                 block3(v25: i64):
;; @0043                               v31 = load.i32 user7 aligned readonly v25+16
;; @0043                               v29 = load.i64 notrap aligned readonly can_move region9 v0+40
;; @0043                               v30 = load.i32 notrap aligned readonly can_move v29+4
;; @0043                               v32 = icmp eq v31, v30
;; @0043                               trapz v32, user8
;; @0043                               v34 = load.i64 notrap aligned readonly v25+8
;; @0043                               v35 = load.i64 notrap aligned readonly v25+24
;; @0043                               v36 = call_indirect sig0, v34(v35, v0, v2)
;; @004a                               v42 = load.i32 little region4 v8
;; @004d                               v44 = load.i64 notrap aligned region7 v10+8
;; @004d                               v49 = load.i64 notrap aligned region6 v10
;; @004d                               v45 = ireduce.i32 v44
;; @004d                               v46 = icmp uge v42, v45
;; @004d                               v47 = uextend.i64 v42
;;                                     v70 = iconst.i64 3
;;                                     v71 = ishl v47, v70  ; v70 = 3
;; @004d                               v52 = iadd v49, v71
;;                                     v72 = iconst.i64 0
;;                                     v73 = select_spectre_guard v46, v72, v52  ; v72 = 0
;; @004d                               v55 = load.i64 user6 aligned region8 v73
;;                                     v74 = iconst.i64 -2
;;                                     v75 = band v55, v74  ; v74 = -2
;; @004d                               brif v55, block5(v75), block4
;;
;;                                 block4 cold:
;;                                     v76 = iconst.i32 0
;; @004d                               v61 = call fn0(v0, v76, v47)  ; v76 = 0
;; @004d                               jump block5(v61)
;;
;;                                 block5(v58: i64):
;; @004d                               v64 = load.i32 user7 aligned readonly v58+16
;; @004d                               v65 = icmp eq v64, v30
;; @004d                               trapz v65, user8
;; @004d                               v67 = load.i64 notrap aligned readonly v58+8
;; @004d                               v68 = load.i64 notrap aligned readonly v58+24
;; @004d                               v69 = call_indirect sig0, v67(v68, v0, v2)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v36, v69
;; }
