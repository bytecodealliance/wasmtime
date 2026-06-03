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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:13 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v6 = load.i64 notrap aligned v0+24
;; @001e                               v7 = load.i64 notrap aligned v6
;; @001e                               v8 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v9 = load.i64 notrap aligned v8+8
;; @001e                               v10 = icmp uge v7, v9
;; @001e                               brif v10, block3, block2(v9)
;;
;;                                 block3 cold:
;; @001e                               v11 = call fn0(v0)
;; @001e                               jump block2(v11)
;;
;;                                 block2(v62: i64):
;; @0025                               v16 = load.i64 notrap aligned v0+64
;; @0025                               v17 = uextend.i64 v2
;; @0025                               v18 = uextend.i64 v4
;; @0025                               v21 = iadd v17, v18
;; @0025                               v22 = icmp ugt v21, v16
;; @0025                               trapnz v22, heap_oob
;; @0025                               v30 = uextend.i64 v3
;; @0025                               v34 = iadd v30, v18
;; @0025                               v35 = icmp ugt v34, v16
;; @0025                               trapnz v35, heap_oob
;; @0025                               v23 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v40 = iadd v23, v30
;; @0025                               v27 = iadd v23, v17
;; @0025                               v43 = icmp ugt v40, v27
;; @0025                               brif v43, block6, block7
;;
;;                                 block4(v45: i64, v46: i64, v47: i64, v50: i64):
;; @0025                               v49 = load.i64 notrap aligned v6
;; @0025                               v51 = icmp uge v49, v50
;; @0025                               brif v51, block9, block8(v50)
;;
;;                                 block5(v89: i64, v90: i64, v91: i64, v95: i64):
;; @0025                               v94 = load.i64 notrap aligned v6
;; @0025                               v97 = icmp uge v94, v95
;; @0025                               brif v97, block17, block16
;;
;;                                 block6:
;;                                     v117 = iconst.i64 0x0800_0000
;;                                     v118 = icmp.i64 ugt v18, v117  ; v117 = 0x0800_0000
;; @0025                               brif v118, block4(v27, v40, v18, v62), block5(v27, v40, v18, v62)
;;
;;                                 block9 cold:
;; @0025                               v53 = load.i64 notrap aligned v8+8
;; @0025                               v54 = icmp.i64 uge v49, v53
;; @0025                               brif v54, block10, block8(v53)
;;
;;                                 block10 cold:
;; @0025                               v55 = call fn0(v0)
;; @0025                               jump block8(v55)
;;
;;                                 block8(v63: i64):
;; @0025                               call fn1(v0, v45, v46, v117)  ; v117 = 0x0800_0000
;; @0025                               v58 = isub.i64 v47, v117  ; v117 = 0x0800_0000
;; @0025                               v59 = icmp ugt v58, v117  ; v117 = 0x0800_0000
;; @0025                               v56 = iadd.i64 v45, v117  ; v117 = 0x0800_0000
;; @0025                               v57 = iadd.i64 v46, v117  ; v117 = 0x0800_0000
;; @0025                               brif v59, block4(v56, v57, v58, v63), block5(v56, v57, v58, v63)
;;
;;                                 block7:
;; @0025                               v42 = iconst.i64 0x0800_0000
;; @0025                               v66 = icmp.i64 ugt v18, v42  ; v42 = 0x0800_0000
;; @0025                               v64 = iadd.i64 v27, v18
;; @0025                               v65 = iadd.i64 v40, v18
;; @0025                               brif v66, block11(v64, v65, v18, v62), block12(v64, v65, v18, v62)
;;
;;                                 block11(v67: i64, v68: i64, v69: i64, v74: i64):
;; @0025                               v73 = load.i64 notrap aligned v6
;; @0025                               v75 = icmp uge v73, v74
;; @0025                               brif v75, block14, block13(v74)
;;
;;                                 block14 cold:
;; @0025                               v77 = load.i64 notrap aligned v8+8
;; @0025                               v78 = icmp.i64 uge v73, v77
;; @0025                               brif v78, block15, block13(v77)
;;
;;                                 block15 cold:
;; @0025                               v79 = call fn0(v0)
;; @0025                               jump block13(v79)
;;
;;                                 block13(v83: i64):
;;                                     v112 = iconst.i64 0x0800_0000
;;                                     v113 = isub.i64 v67, v112  ; v112 = 0x0800_0000
;;                                     v114 = isub.i64 v68, v112  ; v112 = 0x0800_0000
;; @0025                               call fn1(v0, v113, v114, v112)  ; v112 = 0x0800_0000
;;                                     v115 = isub.i64 v69, v112  ; v112 = 0x0800_0000
;;                                     v116 = icmp ugt v115, v112  ; v112 = 0x0800_0000
;; @0025                               brif v116, block11(v113, v114, v115, v83), block12(v113, v114, v115, v83)
;;
;;                                 block12(v84: i64, v85: i64, v86: i64, v96: i64):
;; @0025                               v87 = isub v84, v86
;; @0025                               v88 = isub v85, v86
;; @0025                               jump block5(v87, v88, v86, v96)
;;
;;                                 block17 cold:
;; @0025                               v99 = load.i64 notrap aligned v8+8
;; @0025                               v100 = icmp.i64 uge v94, v99
;; @0025                               brif v100, block18, block16
;;
;;                                 block18 cold:
;; @0025                               v101 = call fn0(v0)
;; @0025                               jump block16
;;
;;                                 block16:
;; @0025                               call fn1(v0, v89, v90, v91)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
