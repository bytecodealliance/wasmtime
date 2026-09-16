;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=null"

;; A field's region is keyed on the type that introduced it. Field 0 comes from
;; `$base`, so reaching it through either type gives one region; field 1 comes
;; from `$derived` and gets another.

(module
  (type $base (sub (struct (field (mut i32)))))
  (type $derived (sub $base (struct (field (mut i32))
                                    (field (mut i32)))))

  (func (param (ref $derived)) (result i32 i32 i32)
    (struct.get $derived 0 (local.get 0))
    (struct.get $base 0 (local.get 0))
    (struct.get $derived 1 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32, i32, i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 147 ""
;;     region5 = 165 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002d                               trapz v2, user16
;; @002d                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002d                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @002d                               v3 = uextend.i64 v2
;; @002d                               v6 = iadd v5, v3
;; @002d                               v7 = iconst.i64 8
;; @002d                               v8 = iadd v6, v7  ; v7 = 8
;; @002d                               v9 = load.i32 user2 little region4 v8
;; @0039                               v21 = iconst.i64 12
;; @0039                               v22 = iadd v6, v21  ; v21 = 12
;; @0039                               v23 = load.i32 user2 little region5 v22
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return v9, v9, v23
;; }
