;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null -O gc-heap-reservation=0 -O gc-heap-guard-size=0"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty)) (result f32)
    (struct.get $ty 0 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_s $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result i32)
    (struct.get_u $ty 1 (local.get 0))
  )

  (func (param (ref null $ty)) (result anyref)
    (struct.get $ty 2 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> f32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0033                               trapz v2, user15
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v5 = iconst.i64 24
;; @0033                               v6 = uadd_overflow_trap v4, v5, user1  ; v5 = 24
;; @0033                               v18 = load.i64 notrap aligned readonly can_move v0+8
;; @0033                               v7 = load.i64 notrap aligned v18+40
;; @0033                               v9 = load.i64 notrap aligned v18+32
;; @0033                               v8 = icmp ugt v6, v7
;; @0033                               v11 = iconst.i64 0
;; @0033                               v10 = iadd v9, v4
;; @0033                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @0033                               v13 = iconst.i64 8
;; @0033                               v14 = iadd v12, v13  ; v13 = 8
;; @0033                               v15 = load.f32 notrap aligned little v14
;; @0037                               jump block1
;;
;;                                 block1:
;; @0037                               return v15
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003c                               trapz v2, user15
;; @003c                               v4 = uextend.i64 v2
;; @003c                               v5 = iconst.i64 24
;; @003c                               v6 = uadd_overflow_trap v4, v5, user1  ; v5 = 24
;; @003c                               v19 = load.i64 notrap aligned readonly can_move v0+8
;; @003c                               v7 = load.i64 notrap aligned v19+40
;; @003c                               v9 = load.i64 notrap aligned v19+32
;; @003c                               v8 = icmp ugt v6, v7
;; @003c                               v11 = iconst.i64 0
;; @003c                               v10 = iadd v9, v4
;; @003c                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @003c                               v13 = iconst.i64 12
;; @003c                               v14 = iadd v12, v13  ; v13 = 12
;; @003c                               v15 = load.i8 notrap aligned little v14
;; @0040                               jump block1
;;
;;                                 block1:
;; @003c                               v16 = sextend.i32 v15
;; @0040                               return v16
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0045                               trapz v2, user15
;; @0045                               v4 = uextend.i64 v2
;; @0045                               v5 = iconst.i64 24
;; @0045                               v6 = uadd_overflow_trap v4, v5, user1  ; v5 = 24
;; @0045                               v19 = load.i64 notrap aligned readonly can_move v0+8
;; @0045                               v7 = load.i64 notrap aligned v19+40
;; @0045                               v9 = load.i64 notrap aligned v19+32
;; @0045                               v8 = icmp ugt v6, v7
;; @0045                               v11 = iconst.i64 0
;; @0045                               v10 = iadd v9, v4
;; @0045                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @0045                               v13 = iconst.i64 12
;; @0045                               v14 = iadd v12, v13  ; v13 = 12
;; @0045                               v15 = load.i8 notrap aligned little v14
;; @0049                               jump block1
;;
;;                                 block1:
;; @0045                               v16 = uextend.i32 v15
;; @0049                               return v16
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004e                               trapz v2, user15
;; @004e                               v4 = uextend.i64 v2
;; @004e                               v5 = iconst.i64 24
;; @004e                               v6 = uadd_overflow_trap v4, v5, user1  ; v5 = 24
;; @004e                               v18 = load.i64 notrap aligned readonly can_move v0+8
;; @004e                               v7 = load.i64 notrap aligned v18+40
;; @004e                               v9 = load.i64 notrap aligned v18+32
;; @004e                               v8 = icmp ugt v6, v7
;; @004e                               v11 = iconst.i64 0
;; @004e                               v10 = iadd v9, v4
;; @004e                               v12 = select_spectre_guard v8, v11, v10  ; v11 = 0
;; @004e                               v13 = iconst.i64 16
;; @004e                               v14 = iadd v12, v13  ; v13 = 16
;; @004e                               v15 = load.i32 notrap aligned little v14
;; @0052                               jump block1
;;
;;                                 block1:
;; @0052                               return v15
;; }
