;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $s (struct))
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param anyref)
    block (result anyref)
      (br_on_cast_fail 0 anyref (ref $s) (local.get 0))
      (call $f)
      return
    end
    (call $g)
    return
  )
)
;; function u0:2(i64 vmctx, i64, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64) tail
;;     fn0 = colocated u1:35 sig0
;;     fn1 = u0:0 sig1
;;     fn2 = u0:1 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v44 = stack_addr.i64 ss0
;;                                     store notrap v2, v44
;;                                     v46 = iconst.i32 0
;; @002e                               v4 = icmp eq v2, v46  ; v46 = 0
;; @002e                               v5 = uextend.i32 v4
;; @002e                               v7 = iconst.i32 1
;;                                     v54 = select v2, v7, v46  ; v7 = 1, v46 = 0
;; @002e                               brif v5, block5(v54), block3
;;
;;                                 block3:
;;                                     v61 = iconst.i32 1
;;                                     v62 = band.i32 v2, v61  ; v61 = 1
;;                                     v63 = iconst.i32 0
;;                                     v64 = select v62, v63, v61  ; v63 = 0, v61 = 1
;; @002e                               brif v62, block5(v64), block4
;;
;;                                 block4:
;; @002e                               v20 = uextend.i64 v2
;; @002e                               v21 = iconst.i64 4
;; @002e                               v22 = uadd_overflow_trap v20, v21, user1  ; v21 = 4
;; @002e                               v23 = iconst.i64 8
;; @002e                               v24 = uadd_overflow_trap v22, v23, user1  ; v23 = 8
;; @002e                               v19 = load.i64 notrap aligned readonly v0+48
;; @002e                               v25 = icmp ule v24, v19
;; @002e                               trapz v25, user1
;; @002e                               v18 = load.i64 notrap aligned readonly v0+40
;; @002e                               v26 = iadd v18, v22
;; @002e                               v27 = load.i32 notrap aligned readonly v26
;; @002e                               v15 = load.i64 notrap aligned readonly v0+80
;; @002e                               v16 = load.i32 notrap aligned readonly v15
;; @002e                               v28 = icmp eq v27, v16
;; @002e                               v29 = uextend.i32 v28
;; @002e                               brif v29, block7(v29), block6
;;
;;                                 block6:
;; @002e                               v31 = call fn0(v0, v27, v16), stack_map=[i32 @ ss0+0]
;; @002e                               jump block7(v31)
;;
;;                                 block7(v32: i32):
;; @002e                               jump block5(v32)
;;
;;                                 block5(v33: i32):
;;                                     v40 = load.i32 notrap v44
;; @002e                               brif v33, block8, block2
;;
;;                                 block8:
;; @0034                               v35 = load.i64 notrap aligned readonly v0+88
;; @0034                               v36 = load.i64 notrap aligned readonly v0+104
;; @0034                               call_indirect sig1, v35(v36, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v38 = load.i64 notrap aligned readonly v0+112
;; @0038                               v39 = load.i64 notrap aligned readonly v0+128
;; @0038                               call_indirect sig2, v38(v39, v0)
;; @003a                               return
;; }
