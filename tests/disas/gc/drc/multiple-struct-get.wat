;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))))

  (func (param (ref null $ty)) (result f32 i32)
    (struct.get $ty 0 (local.get 0))
    (struct.get_s $ty 1 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> f32, i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0023                               trapz v2, user16
;; @0023                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0023                               v7 = load.i64 notrap aligned readonly can_move v6+32
;; @0023                               v5 = uextend.i64 v2
;; @0023                               v8 = iadd v7, v5
;; @0023                               v9 = iconst.i64 24
;; @0023                               v10 = iadd v8, v9  ; v9 = 24
;; @0023                               v11 = load.f32 user2 little region1 v10
;; @0029                               v16 = iconst.i64 28
;; @0029                               v17 = iadd v8, v16  ; v16 = 28
;; @0029                               v18 = load.i8 user2 little region1 v17
;; @002d                               jump block1
;;
;;                                 block1:
;; @0029                               v19 = sextend.i32 v18
;; @002d                               return v11, v19
;; }
