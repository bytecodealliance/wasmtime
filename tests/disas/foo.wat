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
;; @0040                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0040                               v4 = uextend.i64 v3
;; @0040                               v6 = iadd v5, v4
;; @0040                               v7 = load.i32 little region4 v6
;; @0043                               v8 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @0043                               v9 = load.i64 notrap aligned region7 v8+8
;; @0043                               v14 = load.i64 notrap aligned region6 v8
;; @0043                               v10 = ireduce.i32 v9
;; @0043                               v11 = icmp uge v7, v10
;; @0043                               v18 = iconst.i64 0
;; @0043                               v12 = uextend.i64 v7
;; @0043                               v15 = iconst.i64 3
;; @0043                               v16 = ishl v12, v15  ; v15 = 3
;; @0043                               v17 = iadd v14, v16
;; @0043                               v19 = select_spectre_guard v11, v18, v17  ; v18 = 0
;; @0043                               v20 = load.i64 user6 aligned region8 v19
;; @0043                               v21 = iconst.i64 -2
;; @0043                               v22 = band v20, v21  ; v21 = -2
;; @0043                               brif v20, block3(v22), block2
;;
;;                                 block2 cold:
;; @0043                               v24 = iconst.i32 0
;; @0043                               v26 = call fn0(v0, v24, v12)  ; v24 = 0
;; @0043                               jump block3(v26)
;;
;;                                 block3(v23: i64):
;; @0043                               v29 = load.i32 user7 aligned readonly v23+16
;; @0043                               v27 = load.i64 notrap aligned readonly can_move region9 v0+40
;; @0043                               v28 = load.i32 notrap aligned readonly can_move v27+4
;; @0043                               v30 = icmp eq v29, v28
;; @0043                               trapz v30, user8
;; @0043                               v32 = load.i64 notrap aligned readonly v23+8
;; @0043                               v33 = load.i64 notrap aligned readonly v23+24
;; @0043                               v34 = call_indirect sig0, v32(v33, v0, v2)
;; @004a                               v40 = load.i32 little region4 v6
;; @004d                               v42 = load.i64 notrap aligned region7 v8+8
;; @004d                               v47 = load.i64 notrap aligned region6 v8
;; @004d                               v43 = ireduce.i32 v42
;; @004d                               v44 = icmp uge v40, v43
;; @004d                               v45 = uextend.i64 v40
;;                                     v68 = iconst.i64 3
;;                                     v69 = ishl v45, v68  ; v68 = 3
;; @004d                               v50 = iadd v47, v69
;;                                     v70 = iconst.i64 0
;;                                     v71 = select_spectre_guard v44, v70, v50  ; v70 = 0
;; @004d                               v53 = load.i64 user6 aligned region8 v71
;;                                     v72 = iconst.i64 -2
;;                                     v73 = band v53, v72  ; v72 = -2
;; @004d                               brif v53, block5(v73), block4
;;
;;                                 block4 cold:
;;                                     v74 = iconst.i32 0
;; @004d                               v59 = call fn0(v0, v74, v45)  ; v74 = 0
;; @004d                               jump block5(v59)
;;
;;                                 block5(v56: i64):
;; @004d                               v62 = load.i32 user7 aligned readonly v56+16
;; @004d                               v63 = icmp eq v62, v28
;; @004d                               trapz v63, user8
;; @004d                               v65 = load.i64 notrap aligned readonly v56+8
;; @004d                               v66 = load.i64 notrap aligned readonly v56+24
;; @004d                               v67 = call_indirect sig0, v65(v66, v0, v2)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return v34, v67
;; }
