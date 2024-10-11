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
;;                                     v45 = stack_addr.i64 ss0
;;                                     store notrap v2, v45
;;                                     v47 = iconst.i32 0
;; @002f                               v4 = icmp eq v2, v47  ; v47 = 0
;; @002f                               v5 = uextend.i32 v4
;; @002f                               v7 = iconst.i32 1
;;                                     v55 = select v2, v7, v47  ; v7 = 1, v47 = 0
;; @002f                               brif v5, block5(v55), block3
;;
;;                                 block3:
;;                                     v62 = iconst.i32 1
;;                                     v63 = band.i32 v2, v62  ; v62 = 1
;;                                     v64 = iconst.i32 0
;;                                     v65 = select v63, v64, v62  ; v64 = 0, v62 = 1
;; @002f                               brif v63, block5(v65), block4
;;
;;                                 block4:
;; @002f                               v21 = uextend.i64 v2
;; @002f                               v22 = iconst.i64 4
;; @002f                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 4
;; @002f                               v24 = iconst.i64 8
;; @002f                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @002f                               v20 = load.i64 notrap aligned readonly v0+48
;; @002f                               v26 = icmp ule v25, v20
;; @002f                               trapz v26, user1
;; @002f                               v18 = load.i64 notrap aligned readonly v0+40
;; @002f                               v27 = iadd v18, v23
;; @002f                               v28 = load.i32 notrap aligned readonly v27
;; @002f                               v15 = load.i64 notrap aligned readonly v0+80
;; @002f                               v16 = load.i32 notrap aligned readonly v15
;; @002f                               v29 = icmp eq v28, v16
;; @002f                               v30 = uextend.i32 v29
;; @002f                               brif v30, block7(v30), block6
;;
;;                                 block6:
;; @002f                               v32 = call fn0(v0, v28, v16), stack_map=[i32 @ ss0+0]
;; @002f                               jump block7(v32)
;;
;;                                 block7(v33: i32):
;; @002f                               jump block5(v33)
;;
;;                                 block5(v34: i32):
;;                                     v41 = load.i32 notrap v45
;; @002f                               brif v34, block2, block8
;;
;;                                 block8:
;; @0035                               v36 = load.i64 notrap aligned readonly v0+88
;; @0035                               v37 = load.i64 notrap aligned readonly v0+104
;; @0035                               call_indirect sig1, v36(v37, v0)
;; @0037                               return
;;
;;                                 block2:
;; @0039                               v39 = load.i64 notrap aligned readonly v0+112
;; @0039                               v40 = load.i64 notrap aligned readonly v0+128
;; @0039                               call_indirect sig2, v39(v40, v0)
;; @003b                               return
;; }
