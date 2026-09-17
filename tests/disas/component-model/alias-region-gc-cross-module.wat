;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-W function-references,gc -C collector=null -C inlining=n"

;; Core modules in one component are compiled together, so both functions must
;; declare the same `regionN = <user_id>` for field 0 despite declaring the
;; type separately. Inlining is off to keep the region tables comparable.

(component
  (core module $M1
    (type $ty (struct (field (mut i32))))
    (func (export "f") (param (ref $ty)) (result i32)
      (struct.get $ty 0 (local.get 0))
    )
  )

  (core module $M2
    (type $unused (struct (field (mut f64))))
    (type $ty (struct (field (mut i32))))
    (func (export "g") (param (ref $ty)) (result i32)
      (struct.get $ty 0 (local.get 0))
    )
  )

  (core instance $m1 (instantiate $M1))
  (core instance $m2 (instantiate $M2))
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               trapz v2, user16
;; @0031                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0031                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0031                               v3 = uextend.i64 v2
;; @0031                               v6 = iadd v5, v3
;; @0031                               v7 = iconst.i64 8
;; @0031                               v8 = iadd v6, v7  ; v7 = 8
;; @0031                               v9 = load.i32 user2 little region4 v8
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return v9
;; }
;;
;; function u1:0(i64 vmctx, i64, i32) -> i32 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0076                               trapz v2, user16
;; @0076                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0076                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0076                               v3 = uextend.i64 v2
;; @0076                               v6 = iadd v5, v3
;; @0076                               v7 = iconst.i64 8
;; @0076                               v8 = iadd v6, v7  ; v7 = 8
;; @0076                               v9 = load.i32 user2 little region4 v8
;; @007a                               jump block1
;;
;;                                 block1:
;; @007a                               return v9
;; }
