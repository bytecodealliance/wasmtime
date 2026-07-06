;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 67108896 "VMStoreContext+0x20"
;;     region5 = 67108904 "VMStoreContext+0x28"
;;     region6 = 536870912 "GcHeap"
;;     region7 = 1207959576 "VMFunctionImport+0x18"
;;     region8 = 1207959560 "VMFunctionImport+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002e                               v3 = iconst.i32 0
;; @002e                               v4 = icmp eq v2, v3  ; v3 = 0
;; @002e                               brif v4, block5(v3), block3  ; v3 = 0
;;
;;                                 block3:
;; @002e                               v7 = iconst.i32 1
;; @002e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v26 = iconst.i32 0
;; @002e                               brif v8, block5(v26), block4  ; v26 = 0
;;
;;                                 block4:
;; @002e                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002e                               v14 = load.i64 notrap aligned readonly can_move region4 v13+32
;; @002e                               v12 = uextend.i64 v2
;; @002e                               v15 = iadd v14, v12
;; @002e                               v16 = iconst.i64 4
;; @002e                               v17 = iadd v15, v16  ; v16 = 4
;; @002e                               v18 = load.i32 user2 readonly region6 v17
;; @002e                               v10 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @002e                               v11 = load.i32 notrap aligned readonly can_move region3 v10
;; @002e                               v19 = icmp eq v18, v11
;; @002e                               v20 = uextend.i32 v19
;; @002e                               jump block5(v20)
;;
;;                                 block5(v21: i32):
;; @002e                               brif v21, block6, block2
;;
;;                                 block6:
;; @0034                               v23 = load.i64 notrap aligned readonly can_move region8 v0+56
;; @0034                               v22 = load.i64 notrap aligned readonly can_move region7 v0+72
;; @0034                               call_indirect sig0, v23(v22, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v25 = load.i64 notrap aligned readonly can_move region8 v0+88
;; @0038                               v24 = load.i64 notrap aligned readonly can_move region7 v0+104
;; @0038                               call_indirect sig0, v25(v24, v0)
;; @003a                               return
;; }
