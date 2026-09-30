;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=null"

;; Rec-group deduplication collapses the two wasm-level type indices below to
;; one `ModuleInternedTypeIndex`, so both `struct.get`s share a region.

(module
  (type $ty1 (struct (field (mut i32))))
  (type $ty2 (struct (field (mut i32))))

  (func (param (ref $ty1)) (param (ref $ty2)) (result i32 i32)
    (struct.get $ty1 0 (local.get 0))
    (struct.get $ty2 0 (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32, i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 147 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0027                               trapz v2, user16
;; @0027                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0027                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @0027                               v4 = uextend.i64 v2
;; @0027                               v7 = iadd v6, v4
;; @0027                               v8 = iconst.i64 8
;; @0027                               v9 = iadd v7, v8  ; v8 = 8
;; @0027                               v10 = load.i32 user2 little region4 v9
;; @002d                               trapz v3, user16
;; @002d                               v11 = uextend.i64 v3
;; @002d                               v14 = iadd v6, v11
;; @002d                               v16 = iadd v14, v8  ; v8 = 8
;; @002d                               v17 = load.i32 user2 little region4 v16
;; @0031                               jump block1
;;
;;                                 block1:
;; @0031                               return v10, v17
;; }
