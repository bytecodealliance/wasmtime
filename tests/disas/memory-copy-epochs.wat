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
;;     fn0 = colocated u805306368:14 sig0
;;     fn1 = colocated u805306368:4 sig1
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
;;                                 block2(v68: i64):
;; @0025                               v17 = load.i64 notrap aligned v0+64
;; @0025                               v18 = uextend.i64 v2
;; @0025                               v19 = uextend.i64 v4
;; @0025                               v20 = iadd v18, v19
;; @0025                               v21 = icmp ule v20, v17
;; @0025                               trapz v21, heap_oob
;; @0025                               v27 = uextend.i64 v3
;; @0025                               v29 = iadd v27, v19
;; @0025                               v30 = icmp ule v29, v17
;; @0025                               trapz v30, heap_oob
;; @0025                               v36 = iconst.i64 0x0800_0000
;; @0025                               v37 = icmp ugt v19, v36  ; v36 = 0x0800_0000
;; @0025                               v23 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v24 = iadd v23, v18
;; @0025                               v33 = iadd v23, v27
;; @0025                               brif v37, block4(v24, v33, v19, v68), block5(v24, v33, v19, v68)
;;
;;                                 block4(v38: i64, v39: i64, v40: i64, v43: i64):
;; @0025                               v42 = load.i64 notrap aligned v6
;; @0025                               v44 = icmp uge v42, v43
;; @0025                               brif v44, block7, block6(v43)
;;
;;                                 block5(v54: i64, v55: i64, v56: i64, v59: i64):
;; @0025                               v58 = load.i64 notrap aligned v6
;; @0025                               v60 = icmp uge v58, v59
;; @0025                               brif v60, block10, block9
;;
;;                                 block7 cold:
;; @0025                               v46 = load.i64 notrap aligned v8+8
;; @0025                               v47 = icmp.i64 uge v42, v46
;; @0025                               brif v47, block8, block6(v46)
;;
;;                                 block8 cold:
;; @0025                               v49 = call fn0(v0)
;; @0025                               jump block6(v49)
;;
;;                                 block6(v69: i64):
;;                                     v75 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v38, v39, v75)  ; v75 = 0x0800_0000
;;                                     v76 = isub.i64 v40, v75  ; v75 = 0x0800_0000
;;                                     v77 = icmp ugt v76, v75  ; v75 = 0x0800_0000
;;                                     v78 = iadd.i64 v38, v75  ; v75 = 0x0800_0000
;;                                     v79 = iadd.i64 v39, v75  ; v75 = 0x0800_0000
;; @0025                               brif v77, block4(v78, v79, v76, v69), block5(v78, v79, v76, v69)
;;
;;                                 block10 cold:
;; @0025                               v62 = load.i64 notrap aligned v8+8
;; @0025                               v63 = icmp.i64 uge v58, v62
;; @0025                               brif v63, block11, block9
;;
;;                                 block11 cold:
;; @0025                               v65 = call fn0(v0)
;; @0025                               jump block9
;;
;;                                 block9:
;; @0025                               call fn1(v0, v54, v55, v56)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
