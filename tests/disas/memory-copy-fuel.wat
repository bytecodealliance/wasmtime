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
;; @001e                               v7 = iconst.i64 1
;; @001e                               v8 = iadd v6, v7  ; v7 = 1
;; @001e                               v9 = iconst.i64 0
;; @001e                               v10 = icmp sge v8, v9  ; v9 = 0
;; @001e                               brif v10, block2, block3(v8)
;;
;;                                 block2:
;;                                     v96 = iadd.i64 v6, v7  ; v7 = 1
;; @001e                               store notrap aligned v96, v5
;; @001e                               v13 = call fn0(v0)
;; @001e                               v15 = load.i64 notrap aligned v5
;; @001e                               jump block3(v15)
;;
;;                                 block3(v43: i64):
;; @0025                               v20 = load.i64 notrap aligned v0+64
;; @0025                               v21 = uextend.i64 v2
;; @0025                               v22 = uextend.i64 v4
;; @0025                               v24 = iadd v21, v22
;; @0025                               v25 = icmp ugt v24, v20
;; @0025                               trapnz v25, heap_oob
;; @0025                               v32 = uextend.i64 v3
;; @0025                               v35 = iadd v32, v22
;; @0025                               v36 = icmp ugt v35, v20
;; @0025                               trapnz v36, heap_oob
;; @0025                               v46 = iconst.i64 0x0800_0000
;; @0025                               v47 = icmp ugt v22, v46  ; v46 = 0x0800_0000
;; @0025                               v26 = load.i64 notrap aligned readonly can_move v0+56
;; @0025                               v29 = iadd v26, v21
;; @0025                               v40 = iadd v26, v32
;; @0025                               v44 = iconst.i64 4
;; @0025                               v45 = iadd v43, v44  ; v44 = 4
;; @0025                               brif v47, block4(v29, v40, v22, v45), block5(v29, v40, v22, v45)
;;
;;                                 block4(v48: i64, v49: i64, v50: i64, v51: i64):
;;                                     v97 = iconst.i64 0x0800_0000
;;                                     v98 = iadd v51, v97  ; v97 = 0x0800_0000
;;                                     v99 = iconst.i64 0
;;                                     v100 = icmp sge v98, v99  ; v99 = 0
;; @0025                               brif v100, block6, block7(v98)
;;
;;                                 block5(v64: i64, v65: i64, v66: i64, v67: i64):
;; @0025                               v68 = iadd v67, v66
;;                                     v106 = iconst.i64 0
;;                                     v107 = icmp sge v68, v106  ; v106 = 0
;; @0025                               brif v107, block8, block9(v68)
;;
;;                                 block6:
;; @0025                               store.i64 notrap aligned v98, v5
;; @0025                               v57 = call fn0(v0)
;; @0025                               v59 = load.i64 notrap aligned v5
;; @0025                               jump block7(v59)
;;
;;                                 block7(v76: i64):
;;                                     v101 = iconst.i64 0x0800_0000
;; @0025                               call fn1(v0, v48, v49, v101)  ; v101 = 0x0800_0000
;;                                     v102 = isub.i64 v50, v101  ; v101 = 0x0800_0000
;;                                     v103 = icmp ugt v102, v101  ; v101 = 0x0800_0000
;;                                     v104 = iadd.i64 v48, v101  ; v101 = 0x0800_0000
;;                                     v105 = iadd.i64 v49, v101  ; v101 = 0x0800_0000
;; @0025                               brif v103, block4(v104, v105, v102, v76), block5(v104, v105, v102, v76)
;;
;;                                 block8:
;; @0025                               store.i64 notrap aligned v68, v5
;; @0025                               v73 = call fn0(v0)
;; @0025                               v75 = load.i64 notrap aligned v5
;; @0025                               jump block9(v75)
;;
;;                                 block9(v78: i64):
;; @0025                               call fn1(v0, v64, v65, v66)
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               store.i64 notrap aligned v78, v5
;; @0029                               return
;; }
