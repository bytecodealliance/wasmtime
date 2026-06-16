;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $s (struct))
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param anyref)
    block (result (ref $s))
      (br_on_cast 0 anyref (ref $s) (local.get 0))
      (call $f)
      return
    end
    (call $g)
    return
  )
)
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 72 "VMContext+0x48"
;;     region4 = 56 "VMContext+0x38"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002f                               v4 = iconst.i32 0
;; @002f                               v5 = icmp eq v2, v4  ; v4 = 0
;; @002f                               brif v5, block5(v4), block3  ; v4 = 0
;;
;;                                 block3:
;; @002f                               v8 = iconst.i32 1
;; @002f                               v9 = band.i32 v2, v8  ; v8 = 1
;;                                     v27 = iconst.i32 0
;; @002f                               brif v9, block5(v27), block4  ; v27 = 0
;;
;;                                 block4:
;; @002f                               v14 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002f                               v15 = load.i64 notrap aligned readonly can_move v14+32
;; @002f                               v13 = uextend.i64 v2
;; @002f                               v16 = iadd v15, v13
;; @002f                               v17 = iconst.i64 4
;; @002f                               v18 = iadd v16, v17  ; v17 = 4
;; @002f                               v19 = load.i32 user2 readonly region2 v18
;; @002f                               v11 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @002f                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002f                               v20 = icmp eq v19, v12
;; @002f                               v21 = uextend.i32 v20
;; @002f                               jump block5(v21)
;;
;;                                 block5(v22: i32):
;; @002f                               brif v22, block2, block6
;;
;;                                 block6:
;; @0035                               v24 = load.i64 notrap aligned readonly can_move region4 v0+56
;; @0035                               v23 = load.i64 notrap aligned readonly can_move region3 v0+72
;; @0035                               call_indirect sig0, v24(v23, v0)
;; @0037                               return
;;
;;                                 block2:
;; @0039                               v26 = load.i64 notrap aligned readonly can_move region6 v0+88
;; @0039                               v25 = load.i64 notrap aligned readonly can_move region5 v0+104
;; @0039                               call_indirect sig0, v26(v25, v0)
;; @003b                               return
;; }
