;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wfuel=100'

(module
  (memory 1)
  (func $copy (param i32 i32 i32)
    (memory.copy (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435456 "VMStoreContext+0x0"
;;     region3 = 2415919104 "VMMemoryDefinition+0x0"
;;     region4 = 2415919112 "VMMemoryDefinition+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001e                               v6 = load.i64 notrap aligned region2 v5
;; @001e                               v7 = iconst.i64 1
;; @001e                               v8 = iadd v6, v7  ; v7 = 1
;; @001e                               v9 = iconst.i64 0
;; @001e                               v10 = icmp sge v8, v9  ; v9 = 0
;; @001e                               brif v10, block2, block3(v8)
;;
;;                                 block2:
;;                                     v106 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned region2 v106, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned region2 v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v43: i64):
;; @0025                               v18 = load.i64 notrap aligned region4 v0+64
;; @0025                               v19 = uextend.i64 v2
;; @0025                               v20 = uextend.i64 v4
;; @0025                               v23 = iadd v19, v20
;; @0025                               v24 = icmp ugt v23, v18
;; @0025                               trapnz v24, heap_oob
;; @0025                               v31 = uextend.i64 v3
;; @0025                               v35 = iadd v31, v20
;; @0025                               v36 = icmp ugt v35, v18
;; @0025                               trapnz v36, heap_oob
;; @0025                               v25 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0025                               v41 = iadd v25, v31
;; @0025                               v29 = iadd v25, v19
;; @0025                               v47 = icmp ugt v41, v29
;; @0025                               brif v47, block6, block7
;;
;;                                 block4(v49: i64, v50: i64, v51: i64, v52: i64):
;; @0025                               v53 = iadd v52, v116  ; v116 = 0x0800_0000
;;                                     v120 = iconst.i64 0
;;                                     v121 = icmp sge v53, v120  ; v120 = 0
;; @0025                               brif v121, block8, block9(v53)
;;
;;                                 block5(v89: i64, v90: i64, v91: i64, v92: i64):
;; @0025                               v94 = iadd v92, v91
;;                                     v123 = iconst.i64 0
;;                                     v124 = icmp sge v94, v123  ; v123 = 0
;; @0025                               brif v124, block14, block15(v94)
;;
;;                                 block6:
;;                                     v116 = iconst.i64 0x0800_0000
;;                                     v117 = icmp.i64 ugt v20, v116  ; v116 = 0x0800_0000
;;                                     v118 = iconst.i64 4
;;                                     v119 = iadd.i64 v43, v118  ; v118 = 4
;; @0025                               brif v117, block4(v29, v41, v20, v119), block5(v29, v41, v20, v119)
;;
;;                                 block8:
;;                                     v122 = iadd.i64 v52, v116  ; v116 = 0x0800_0000
;; @0025                               store notrap aligned region2 v122, v5
;; @0025                               v57 = call fn0(v0)
;; @0025                               v59 = load.i64 notrap aligned region2 v5
;; @0025                               jump block9(v59)
;;
;;                                 block9(v64: i64):
;; @0025                               call fn1(v0, v49, v50, v116)  ; v116 = 0x0800_0000
;; @0025                               v62 = isub.i64 v51, v116  ; v116 = 0x0800_0000
;; @0025                               v63 = icmp ugt v62, v116  ; v116 = 0x0800_0000
;; @0025                               v60 = iadd.i64 v49, v116  ; v116 = 0x0800_0000
;; @0025                               v61 = iadd.i64 v50, v116  ; v116 = 0x0800_0000
;; @0025                               brif v63, block4(v60, v61, v62, v64), block5(v60, v61, v62, v64)
;;
;;                                 block7:
;; @0025                               v46 = iconst.i64 0x0800_0000
;; @0025                               v67 = icmp.i64 ugt v20, v46  ; v46 = 0x0800_0000
;; @0025                               v65 = iadd.i64 v29, v20
;; @0025                               v66 = iadd.i64 v41, v20
;; @0025                               v44 = iconst.i64 4
;; @0025                               v45 = iadd.i64 v43, v44  ; v44 = 4
;; @0025                               brif v67, block10(v65, v66, v20, v45), block11(v65, v66, v20, v45)
;;
;;                                 block10(v68: i64, v69: i64, v70: i64, v73: i64):
;;                                     v107 = iconst.i64 0x0800_0000
;;                                     v108 = iadd v73, v107  ; v107 = 0x0800_0000
;;                                     v109 = iconst.i64 0
;;                                     v110 = icmp sge v108, v109  ; v109 = 0
;; @0025                               brif v110, block12, block13(v108)
;;
;;                                 block12:
;; @0025                               store.i64 notrap aligned region2 v108, v5
;; @0025                               v78 = call fn0(v0)
;; @0025                               v80 = load.i64 notrap aligned region2 v5
;; @0025                               jump block13(v80)
;;
;;                                 block13(v83: i64):
;;                                     v111 = iconst.i64 0x0800_0000
;;                                     v112 = isub.i64 v68, v111  ; v111 = 0x0800_0000
;;                                     v113 = isub.i64 v69, v111  ; v111 = 0x0800_0000
;; @0025                               call fn1(v0, v112, v113, v111)  ; v111 = 0x0800_0000
;;                                     v114 = isub.i64 v70, v111  ; v111 = 0x0800_0000
;;                                     v115 = icmp ugt v114, v111  ; v111 = 0x0800_0000
;; @0025                               brif v115, block10(v112, v113, v114, v83), block11(v112, v113, v114, v83)
;;
;;                                 block11(v84: i64, v85: i64, v86: i64, v93: i64):
;; @0025                               v87 = isub v84, v86
;; @0025                               v88 = isub v85, v86
;; @0025                               jump block5(v87, v88, v86, v93)
;;
;;                                 block14:
;; @0025                               store.i64 notrap aligned region2 v94, v5
;; @0025                               v98 = call fn0(v0)
;; @0025                               v100 = load.i64 notrap aligned region2 v5
;; @0025                               jump block15(v100)
;;
;;                                 block15(v102: i64):
;; @0025                               call fn1(v0, v89, v90, v91)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned region2 v102, v5
;; @0029                               return
;; }
