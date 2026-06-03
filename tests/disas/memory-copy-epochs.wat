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
;;                                 block2(v76: i64):
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
;; @0025                               v44 = iconst.i64 0x0800_0000
;; @0025                               v45 = icmp ugt v19, v44  ; v44 = 0x0800_0000
;; @0025                               v24 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v28 = iadd v24, v18
;; @0025                               v41 = iadd v24, v31
;; @0025                               brif v45, block4(v28, v41, v19, v76), block5(v28, v41, v19, v76)
;;
;;                                 block4(v46: i64, v47: i64, v48: i64, v51: i64):
;; @0025                               v50 = load.i64 notrap aligned v6
;; @0025                               v52 = icmp uge v50, v51
;; @0025                               brif v52, block7, block6(v51)
;;
;;                                 block5(v62: i64, v63: i64, v64: i64, v67: i64):
;; @0025                               v66 = load.i64 notrap aligned v6
;; @0025                               v68 = icmp uge v66, v67
;; @0025                               brif v68, block10, block9
;;
;;                                 block7 cold:
;; @0025                               v54 = load.i64 notrap aligned v8+8
;; @0025                               v55 = icmp.i64 uge v50, v54
;; @0025                               brif v55, block8, block6(v54)
;;
;;                                 block8 cold:
;; @0025                               v57 = call fn0(v0)
;; @0025                               jump block6(v57)
;;
;;                                 block6(v77: i64):
;;                                     v87 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v46, v47, v87)  ; v87 = 0x0800_0000
;;                                     v88 = isub.i64 v48, v87  ; v87 = 0x0800_0000
;;                                     v89 = icmp ugt v88, v87  ; v87 = 0x0800_0000
;;                                     v90 = iadd.i64 v46, v87  ; v87 = 0x0800_0000
;;                                     v91 = iadd.i64 v47, v87  ; v87 = 0x0800_0000
;; @0025                               brif v89, block4(v90, v91, v88, v77), block5(v90, v91, v88, v77)
;;
;;                                 block10 cold:
;; @0025                               v70 = load.i64 notrap aligned v8+8
;; @0025                               v71 = icmp.i64 uge v66, v70
;; @0025                               brif v71, block11, block9
;;
;;                                 block11 cold:
;; @0025                               v73 = call fn0(v0)
;; @0025                               jump block9
;;
;;                                 block9:
;; @0025                               call fn1(v0, v62, v63, v64)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
