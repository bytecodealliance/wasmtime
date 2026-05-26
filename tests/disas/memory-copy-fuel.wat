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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv3+64
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx) -> i8 tail
;;     sig1 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:12 sig0
;;     fn1 = colocated u805306368:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @001e                               v5 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v6 = load.i64 notrap aligned v5
;;                                     v91 = iconst.i64 1
;; @001e                               v7 = iadd v6, v91  ; v91 = 1
;; @001e                               v8 = iconst.i64 0
;; @001e                               v9 = icmp sge v7, v8  ; v8 = 0
;; @001e                               brif v9, block2, block3(v7)
;;
;;                                 block2:
;;                                     v96 = iadd.i64 v6, v91  ; v91 = 1
;; @001e                               store notrap aligned v96, v5
;; @001e                               v12 = call fn0(v0)
;; @001e                               v14 = load.i64 notrap aligned v5
;; @001e                               jump block3(v14)
;;
;;                                 block3(v42: i64):
;; @0025                               v19 = load.i64 notrap aligned v0+64
;; @0025                               v20 = uextend.i64 v2
;; @0025                               v21 = uextend.i64 v4
;; @0025                               v23 = iadd v20, v21
;; @0025                               v24 = icmp ugt v23, v19
;; @0025                               trapnz v24, heap_oob
;; @0025                               v31 = uextend.i64 v3
;; @0025                               v34 = iadd v31, v21
;; @0025                               v35 = icmp ugt v34, v19
;; @0025                               trapnz v35, heap_oob
;; @0025                               v44 = iconst.i64 0x0800_0000
;; @0025                               v45 = icmp ugt v21, v44  ; v44 = 0x0800_0000
;; @0025                               v25 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v28 = iadd v25, v20
;; @0025                               v39 = iadd v25, v31
;;                                     v82 = iconst.i64 4
;; @0025                               v43 = iadd v42, v82  ; v82 = 4
;; @0025                               brif v45, block4(v28, v39, v21, v43), block5(v28, v39, v21, v43)
;;
;;                                 block4(v46: i64, v47: i64, v48: i64, v49: i64):
;;                                     v97 = iconst.i64 0x0800_0000
;;                                     v98 = iadd v49, v97  ; v97 = 0x0800_0000
;;                                     v99 = iconst.i64 0
;;                                     v100 = icmp sge v98, v99  ; v99 = 0
;; @0025                               brif v100, block6, block7(v98)
;;
;;                                 block5(v62: i64, v63: i64, v64: i64, v65: i64):
;; @0025                               v66 = iadd v65, v64
;;                                     v106 = iconst.i64 0
;;                                     v107 = icmp sge v66, v106  ; v106 = 0
;; @0025                               brif v107, block8, block9(v66)
;;
;;                                 block6:
;; @0025                               store.i64 notrap aligned v98, v5
;; @0025                               v55 = call fn0(v0)
;; @0025                               v57 = load.i64 notrap aligned v5
;; @0025                               jump block7(v57)
;;
;;                                 block7(v74: i64):
;;                                     v101 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v46, v47, v101)  ; v101 = 0x0800_0000
;;                                     v102 = isub.i64 v48, v101  ; v101 = 0x0800_0000
;;                                     v103 = icmp ugt v102, v101  ; v101 = 0x0800_0000
;;                                     v104 = iadd.i64 v46, v101  ; v101 = 0x0800_0000
;;                                     v105 = iadd.i64 v47, v101  ; v101 = 0x0800_0000
;; @0025                               brif v103, block4(v104, v105, v102, v74), block5(v104, v105, v102, v74)
;;
;;                                 block8:
;; @0025                               store.i64 notrap aligned v66, v5
;; @0025                               v71 = call fn0(v0)
;; @0025                               v73 = load.i64 notrap aligned v5
;; @0025                               jump block9(v73)
;;
;;                                 block9(v76: i64):
;; @0025                               call fn1(v0, v62, v63, v64)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v76, v5
;; @0029                               return
;; }
