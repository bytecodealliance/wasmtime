;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wepoch-interruption'

(module
  (memory 1)
  (func $copy (param i32 i32 i32)
    (memory.copy (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 24 "VMContext+0x18"
;;     region3 = 268435464 "VMStoreContext+0x8"
;;     region4 = 2415919104 "VMMemoryDefinition+0x0"
;;     region5 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned region2 v0+24
;; @001e                               v6 = load.i64 notrap aligned v5
;; @001e                               v7 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v8 = load.i64 notrap aligned region3 v7+8
;; @001e                               v9 = icmp uge v6, v8
;; @001e                               brif v9, block3, block2(v8)
;;
;;                                 block3 cold:
;; @001e                               v10 = call fn0(v0)
;; @001e                               jump block2(v10)
;;
;;                                 block2(v59: i64):
;; @0025                               v14 = load.i64 notrap aligned region5 v0+64
;; @0025                               v15 = uextend.i64 v2
;; @0025                               v16 = uextend.i64 v4
;; @0025                               v19 = iadd v15, v16
;; @0025                               v20 = icmp ugt v19, v14
;; @0025                               trapnz v20, heap_oob
;; @0025                               v27 = uextend.i64 v3
;; @0025                               v31 = iadd v27, v16
;; @0025                               v32 = icmp ugt v31, v14
;; @0025                               trapnz v32, heap_oob
;; @0025                               v21 = load.i64 notrap aligned readonly can_move region4 v0+56
;; @0025                               v37 = iadd v21, v27
;; @0025                               v25 = iadd v21, v15
;; @0025                               v40 = icmp ugt v37, v25
;; @0025                               brif v40, block6, block7
;;
;;                                 block4(v42: i64, v43: i64, v44: i64, v47: i64):
;; @0025                               v46 = load.i64 notrap aligned v5
;; @0025                               v48 = icmp uge v46, v47
;; @0025                               brif v48, block9, block8(v47)
;;
;;                                 block5(v86: i64, v87: i64, v88: i64, v92: i64):
;; @0025                               v91 = load.i64 notrap aligned v5
;; @0025                               v94 = icmp uge v91, v92
;; @0025                               brif v94, block17, block16
;;
;;                                 block6:
;;                                     v108 = iconst.i64 0x0800_0000
;;                                     v109 = icmp.i64 ugt v16, v108  ; v108 = 0x0800_0000
;; @0025                               brif v109, block4(v25, v37, v16, v59), block5(v25, v37, v16, v59)
;;
;;                                 block9 cold:
;; @0025                               v50 = load.i64 notrap aligned region3 v7+8
;; @0025                               v51 = icmp.i64 uge v46, v50
;; @0025                               brif v51, block10, block8(v50)
;;
;;                                 block10 cold:
;; @0025                               v52 = call fn0(v0)
;; @0025                               jump block8(v52)
;;
;;                                 block8(v60: i64):
;; @0025                               call fn1(v0, v42, v43, v108)  ; v108 = 0x0800_0000
;; @0025                               v55 = isub.i64 v44, v108  ; v108 = 0x0800_0000
;; @0025                               v56 = icmp ugt v55, v108  ; v108 = 0x0800_0000
;; @0025                               v53 = iadd.i64 v42, v108  ; v108 = 0x0800_0000
;; @0025                               v54 = iadd.i64 v43, v108  ; v108 = 0x0800_0000
;; @0025                               brif v56, block4(v53, v54, v55, v60), block5(v53, v54, v55, v60)
;;
;;                                 block7:
;; @0025                               v39 = iconst.i64 0x0800_0000
;; @0025                               v63 = icmp.i64 ugt v16, v39  ; v39 = 0x0800_0000
;; @0025                               v61 = iadd.i64 v25, v16
;; @0025                               v62 = iadd.i64 v37, v16
;; @0025                               brif v63, block11(v61, v62, v16, v59), block12(v61, v62, v16, v59)
;;
;;                                 block11(v64: i64, v65: i64, v66: i64, v71: i64):
;; @0025                               v70 = load.i64 notrap aligned v5
;; @0025                               v72 = icmp uge v70, v71
;; @0025                               brif v72, block14, block13(v71)
;;
;;                                 block14 cold:
;; @0025                               v74 = load.i64 notrap aligned region3 v7+8
;; @0025                               v75 = icmp.i64 uge v70, v74
;; @0025                               brif v75, block15, block13(v74)
;;
;;                                 block15 cold:
;; @0025                               v76 = call fn0(v0)
;; @0025                               jump block13(v76)
;;
;;                                 block13(v80: i64):
;;                                     v103 = iconst.i64 0x0800_0000
;;                                     v104 = isub.i64 v64, v103  ; v103 = 0x0800_0000
;;                                     v105 = isub.i64 v65, v103  ; v103 = 0x0800_0000
;; @0025                               call fn1(v0, v104, v105, v103)  ; v103 = 0x0800_0000
;;                                     v106 = isub.i64 v66, v103  ; v103 = 0x0800_0000
;;                                     v107 = icmp ugt v106, v103  ; v103 = 0x0800_0000
;; @0025                               brif v107, block11(v104, v105, v106, v80), block12(v104, v105, v106, v80)
;;
;;                                 block12(v81: i64, v82: i64, v83: i64, v93: i64):
;; @0025                               v84 = isub v81, v83
;; @0025                               v85 = isub v82, v83
;; @0025                               jump block5(v84, v85, v83, v93)
;;
;;                                 block17 cold:
;; @0025                               v96 = load.i64 notrap aligned region3 v7+8
;; @0025                               v97 = icmp.i64 uge v91, v96
;; @0025                               brif v97, block18, block16
;;
;;                                 block18 cold:
;; @0025                               v98 = call fn0(v0)
;; @0025                               jump block16
;;
;;                                 block16:
;; @0025                               call fn1(v0, v86, v87, v88)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
