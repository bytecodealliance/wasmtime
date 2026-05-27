;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v74 = iconst.i64 3
;;                                     v75 = ishl v6, v74  ; v74 = 3
;;                                     v72 = iconst.i64 32
;; @0022                               v8 = ushr v75, v72  ; v72 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v81 = iconst.i32 3
;;                                     v82 = ishl v3, v81  ; v81 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v82, user18  ; v5 = 32
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v79 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v12, v15, v10, v79)  ; v12 = -1476395008, v79 = 8
;;                                     v71 = stack_addr.i64 ss0
;;                                     store notrap v17, v71
;; @0022                               v69 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move v69+32
;; @0022                               v19 = uextend.i64 v17
;; @0022                               v20 = iadd v18, v19
;;                                     v68 = iconst.i64 24
;; @0022                               v21 = iadd v20, v68  ; v68 = 24
;; @0022                               store user2 v3, v21
;; @0022                               trapz v17, user16
;; @0022                               v45 = load.i64 notrap aligned v69+40
;; @0022                               v38 = iadd v20, v72  ; v72 = 32
;; @0022                               v47 = uadd_overflow_trap v38, v75, user2
;; @0022                               v46 = iadd v18, v45
;; @0022                               v48 = icmp ugt v47, v46
;; @0022                               trapnz v48, user2
;;                                     v85 = iconst.i64 0
;; @0022                               v50 = icmp eq v6, v85  ; v85 = 0
;;                                     v73 = iconst.i64 8
;; @0022                               v49 = iadd v38, v75
;; @0022                               brif v50, block3, block2(v38)
;;
;;                                 block2(v51: i64):
;; @0022                               store.i64 user2 little v2, v51
;;                                     v100 = iconst.i64 8
;;                                     v101 = iadd v51, v100  ; v100 = 8
;; @0022                               v53 = icmp eq v101, v49
;; @0022                               brif v53, block3, block2(v101)
;;
;;                                 block3:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
