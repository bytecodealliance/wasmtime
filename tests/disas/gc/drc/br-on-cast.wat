;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 239 ""
;;     region5 = 130 ""
;;     region6 = 6 ""
;;     region7 = 134 ""
;;     region8 = 25 ""
;;     region9 = 68 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002f                               v3 = iconst.i32 0
;; @002f                               v4 = icmp eq v2, v3  ; v3 = 0
;; @002f                               brif v4, block5(v3), block3  ; v3 = 0
;;
;;                                 block3:
;; @002f                               v7 = iconst.i32 1
;; @002f                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v38 = iconst.i32 0
;; @002f                               brif v8, block5(v38), block4  ; v38 = 0
;;
;;                                 block4:
;; @002f                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002f                               v12 = load.i64 notrap aligned readonly can_move region2 v11+32
;; @002f                               v10 = uextend.i64 v2
;; @002f                               v13 = iadd v12, v10
;; @002f                               v16 = load.i32 user2 readonly region4 v13
;; @002f                               v17 = iconst.i32 -1342177280
;; @002f                               v18 = band v16, v17  ; v17 = -1342177280
;; @002f                               v19 = icmp eq v18, v17  ; v17 = -1342177280
;;                                     v39 = iconst.i32 0
;; @002f                               brif v19, block6, block5(v39)  ; v39 = 0
;;
;;                                 block6:
;; @002f                               v28 = iconst.i64 4
;; @002f                               v29 = iadd.i64 v13, v28  ; v28 = 4
;; @002f                               v30 = load.i32 user2 readonly region7 v29
;; @002f                               v22 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @002f                               v23 = load.i32 notrap aligned readonly can_move region6 v22
;; @002f                               v31 = icmp eq v30, v23
;; @002f                               v32 = uextend.i32 v31
;; @002f                               jump block5(v32)
;;
;;                                 block5(v33: i32):
;; @002f                               brif v33, block2, block7
;;
;;                                 block7:
;; @0035                               v35 = load.i64 notrap aligned readonly can_move region9 v0+56
;; @0035                               v34 = load.i64 notrap aligned readonly can_move region8 v0+72
;; @0035                               call_indirect sig0, v35(v34, v0)
;; @0037                               return
;;
;;                                 block2:
;; @0039                               v37 = load.i64 notrap aligned readonly can_move region9 v0+88
;; @0039                               v36 = load.i64 notrap aligned readonly can_move region8 v0+104
;; @0039                               call_indirect sig0, v37(v36, v0)
;; @003b                               return
;; }
