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
;;                                 block2(v72: i64):
;; @0025                               v17 = load.i64 notrap aligned v0+64
;; @0025                               v18 = uextend.i64 v2
;; @0025                               v19 = uextend.i64 v4
;; @0025                               v21 = iadd v18, v19
;; @0025                               v22 = icmp ugt v21, v17
;; @0025                               trapnz v22, heap_oob
;; @0025                               v29 = uextend.i64 v3
;; @0025                               v32 = iadd v29, v19
;; @0025                               v33 = icmp ugt v32, v17
;; @0025                               trapnz v33, heap_oob
;; @0025                               v40 = iconst.i64 0x0800_0000
;; @0025                               v41 = icmp ugt v19, v40  ; v40 = 0x0800_0000
;; @0025                               v23 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v26 = iadd v23, v18
;; @0025                               v37 = iadd v23, v29
;; @0025                               brif v41, block4(v26, v37, v19, v72), block5(v26, v37, v19, v72)
;;
;;                                 block4(v42: i64, v43: i64, v44: i64, v47: i64):
;; @0025                               v46 = load.i64 notrap aligned v6
;; @0025                               v48 = icmp uge v46, v47
;; @0025                               brif v48, block7, block6(v47)
;;
;;                                 block5(v58: i64, v59: i64, v60: i64, v63: i64):
;; @0025                               v62 = load.i64 notrap aligned v6
;; @0025                               v64 = icmp uge v62, v63
;; @0025                               brif v64, block10, block9
;;
;;                                 block7 cold:
;; @0025                               v50 = load.i64 notrap aligned v8+8
;; @0025                               v51 = icmp.i64 uge v46, v50
;; @0025                               brif v51, block8, block6(v50)
;;
;;                                 block8 cold:
;; @0025                               v53 = call fn0(v0)
;; @0025                               jump block6(v53)
;;
;;                                 block6(v73: i64):
;;                                     v87 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v42, v43, v87)  ; v87 = 0x0800_0000
;;                                     v88 = isub.i64 v44, v87  ; v87 = 0x0800_0000
;;                                     v89 = icmp ugt v88, v87  ; v87 = 0x0800_0000
;;                                     v90 = iadd.i64 v42, v87  ; v87 = 0x0800_0000
;;                                     v91 = iadd.i64 v43, v87  ; v87 = 0x0800_0000
;; @0025                               brif v89, block4(v90, v91, v88, v73), block5(v90, v91, v88, v73)
;;
;;                                 block10 cold:
;; @0025                               v66 = load.i64 notrap aligned v8+8
;; @0025                               v67 = icmp.i64 uge v62, v66
;; @0025                               brif v67, block11, block9
;;
;;                                 block11 cold:
;; @0025                               v69 = call fn0(v0)
;; @0025                               jump block9
;;
;;                                 block9:
;; @0025                               call fn1(v0, v58, v59, v60)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return
;; }
