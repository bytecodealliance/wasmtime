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
;; @001e                               v12 = call fn0(v0)
;; @001e                               jump block2(v12)
;;
;;                                 block2(v65: i64):
;; @0025                               v17 = load.i64 notrap aligned v0+64
;; @0025                               v18 = uextend.i64 v2
;; @0025                               v19 = uextend.i64 v4
;; @0025                               v22 = iadd v18, v19
;; @0025                               v23 = icmp ugt v22, v17
;; @0025                               trapnz v23, heap_oob
;; @0025                               v31 = uextend.i64 v3
;; @0025                               v35 = iadd v31, v19
;; @0025                               v36 = icmp ugt v35, v17
;; @0025                               trapnz v36, heap_oob
;; @0025                               v24 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v41 = iadd v24, v31
;; @0025                               v28 = iadd v24, v18
;; @0025                               v45 = icmp ugt v41, v28
;; @0025                               brif v45, block6, block7
;;
;;                                 block4(v47: i64, v48: i64, v49: i64, v52: i64):
;; @0025                               v51 = load.i64 notrap aligned v6
;; @0025                               v53 = icmp uge v51, v52
;; @0025                               brif v53, block9, block8(v52)
;;
;;                                 block5(v93: i64, v94: i64, v95: i64, v99: i64):
;; @0025                               v98 = load.i64 notrap aligned v6
;; @0025                               v101 = icmp uge v98, v99
;; @0025                               brif v101, block17, block16
;;
;;                                 block6:
;;                                     v122 = iconst.i64 0x0800_0000
;;                                     v123 = icmp.i64 ugt v19, v122  ; v122 = 0x0800_0000
;; @0025                               brif v123, block4(v28, v41, v19, v65), block5(v28, v41, v19, v65)
;;
;;                                 block9 cold:
;; @0025                               v55 = load.i64 notrap aligned v8+8
;; @0025                               v56 = icmp.i64 uge v51, v55
;; @0025                               brif v56, block10, block8(v55)
;;
;;                                 block10 cold:
;; @0025                               v58 = call fn0(v0)
;; @0025                               jump block8(v58)
;;
;;                                 block8(v66: i64):
;; @0025                               call fn1(v0, v47, v48, v122)  ; v122 = 0x0800_0000
;; @0025                               v61 = isub.i64 v49, v122  ; v122 = 0x0800_0000
;; @0025                               v62 = icmp ugt v61, v122  ; v122 = 0x0800_0000
;; @0025                               v59 = iadd.i64 v47, v122  ; v122 = 0x0800_0000
;; @0025                               v60 = iadd.i64 v48, v122  ; v122 = 0x0800_0000
;; @0025                               brif v62, block4(v59, v60, v61, v66), block5(v59, v60, v61, v66)
;;
;;                                 block7:
;; @0025                               v44 = iconst.i64 0x0800_0000
;; @0025                               v69 = icmp.i64 ugt v19, v44  ; v44 = 0x0800_0000
;; @0025                               v67 = iadd.i64 v28, v19
;; @0025                               v68 = iadd.i64 v41, v19
;; @0025                               brif v69, block11(v67, v68, v19, v65), block12(v67, v68, v19, v65)
;;
;;                                 block11(v70: i64, v71: i64, v72: i64, v77: i64):
;; @0025                               v76 = load.i64 notrap aligned v6
;; @0025                               v78 = icmp uge v76, v77
;; @0025                               brif v78, block14, block13(v77)
;;
;;                                 block14 cold:
;; @0025                               v80 = load.i64 notrap aligned v8+8
;; @0025                               v81 = icmp.i64 uge v76, v80
;; @0025                               brif v81, block15, block13(v80)
;;
;;                                 block15 cold:
;; @0025                               v83 = call fn0(v0)
;; @0025                               jump block13(v83)
;;
;;                                 block13(v87: i64):
;;                                     v117 = iconst.i64 0x0800_0000
;;                                     v118 = isub.i64 v70, v117  ; v117 = 0x0800_0000
;;                                     v119 = isub.i64 v71, v117  ; v117 = 0x0800_0000
;; @0025                               call fn1(v0, v118, v119, v117)  ; v117 = 0x0800_0000
;;                                     v120 = isub.i64 v72, v117  ; v117 = 0x0800_0000
;;                                     v121 = icmp ugt v120, v117  ; v117 = 0x0800_0000
;; @0025                               brif v121, block11(v118, v119, v120, v87), block12(v118, v119, v120, v87)
;;
;;                                 block12(v88: i64, v89: i64, v90: i64, v100: i64):
;; @0025                               v91 = isub v88, v90
;; @0025                               v92 = isub v89, v90
;; @0025                               jump block5(v91, v92, v90, v100)
;;
;;                                 block17 cold:
;; @0025                               v103 = load.i64 notrap aligned v8+8
;; @0025                               v104 = icmp.i64 uge v98, v103
;; @0025                               brif v104, block18, block16
;;
;;                                 block18 cold:
;; @0025                               v106 = call fn0(v0)
;; @0025                               jump block16
;;
;;                                 block16:
;; @0025                               call fn1(v0, v93, v94, v95)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
