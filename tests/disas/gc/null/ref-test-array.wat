;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (func (param anyref) (result i32)
    (ref.test (ref array) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v23 = iconst.i32 0
;; @001b                               v4 = icmp eq v2, v23  ; v23 = 0
;; @001b                               v5 = uextend.i32 v4
;; @001b                               brif v5, block4(v23), block2  ; v23 = 0
;;
;;                                 block2:
;; @001b                               v7 = iconst.i32 1
;; @001b                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v24 = iconst.i32 0
;; @001b                               brif v8, block4(v24), block3  ; v24 = 0
;;
;;                                 block3:
;; @001b                               v21 = load.i64 notrap aligned readonly can_move v0+8
;; @001b                               v11 = load.i64 notrap aligned readonly can_move v21+24
;; @001b                               v10 = uextend.i64 v2
;; @001b                               v12 = iadd v11, v10
;; @001b                               v15 = load.i32 notrap aligned readonly v12
;; @001b                               v16 = iconst.i32 -1476395008
;; @001b                               v17 = band v15, v16  ; v16 = -1476395008
;; @001b                               v18 = icmp eq v17, v16  ; v16 = -1476395008
;; @001b                               v19 = uextend.i32 v18
;; @001b                               jump block4(v19)
;;
;;                                 block4(v20: i32):
;; @001e                               jump block1(v20)
;;
;;                                 block1(v3: i32):
;; @001e                               return v3
;; }
