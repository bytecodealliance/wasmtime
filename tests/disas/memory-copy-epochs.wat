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
;;     fn1 = colocated u805306368:3 sig1
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
;;                                 block2(v70: i64):
;; @0025                               v17 = load.i64 notrap aligned v0+64
;; @0025                               v18 = uextend.i64 v2
;; @0025                               v19 = uextend.i64 v4
;; @0025                               v20 = iadd v18, v19
;; @0025                               v21 = icmp ugt v20, v17
;; @0025                               trapnz v21, heap_oob
;; @0025                               v28 = uextend.i64 v3
;; @0025                               v30 = iadd v28, v19
;; @0025                               v31 = icmp ugt v30, v17
;; @0025                               trapnz v31, heap_oob
;; @0025                               v38 = iconst.i64 0x0800_0000
;; @0025                               v39 = icmp ugt v19, v38  ; v38 = 0x0800_0000
;; @0025                               v22 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v25 = iadd v22, v18
;; @0025                               v35 = iadd v22, v28
;; @0025                               brif v39, block4(v25, v35, v19, v70), block5(v25, v35, v19, v70)
;;
;;                                 block4(v40: i64, v41: i64, v42: i64, v45: i64):
;; @0025                               v44 = load.i64 notrap aligned v6
;; @0025                               v46 = icmp uge v44, v45
;; @0025                               brif v46, block7, block6(v45)
;;
;;                                 block5(v56: i64, v57: i64, v58: i64, v61: i64):
;; @0025                               v60 = load.i64 notrap aligned v6
;; @0025                               v62 = icmp uge v60, v61
;; @0025                               brif v62, block10, block9
;;
;;                                 block7 cold:
;; @0025                               v48 = load.i64 notrap aligned v8+8
;; @0025                               v49 = icmp.i64 uge v44, v48
;; @0025                               brif v49, block8, block6(v48)
;;
;;                                 block8 cold:
;; @0025                               v51 = call fn0(v0)
;; @0025                               jump block6(v51)
;;
;;                                 block6(v71: i64):
;;                                     v82 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v40, v41, v82)  ; v82 = 0x0800_0000
;;                                     v83 = isub.i64 v42, v82  ; v82 = 0x0800_0000
;;                                     v84 = icmp ugt v83, v82  ; v82 = 0x0800_0000
;;                                     v85 = iadd.i64 v40, v82  ; v82 = 0x0800_0000
;;                                     v86 = iadd.i64 v41, v82  ; v82 = 0x0800_0000
;; @0025                               brif v84, block4(v85, v86, v83, v71), block5(v85, v86, v83, v71)
;;
;;                                 block10 cold:
;; @0025                               v64 = load.i64 notrap aligned v8+8
;; @0025                               v65 = icmp.i64 uge v60, v64
;; @0025                               brif v65, block11, block9
;;
;;                                 block11 cold:
;; @0025                               v67 = call fn0(v0)
;; @0025                               jump block9
;;
;;                                 block9:
;; @0025                               call fn1(v0, v56, v57, v58)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
