;;! target = "riscv64"
;;! test = 'optimize'
;;! filter = 'wasm[2]--function[1]'

(component
  (type $a (enum "a" "b" "c"))
  (type $func_ty (func (param "x" $a)))

  (component $c1
    (import "a" (type $a' (eq $a)))
    (core module $m1
      (func (export "f") (result i32)
        (i32.const 0)))
    (core instance $ci1 (instantiate $m1))
    (func (export "f") (result $a') (canon lift (core func $ci1 "f"))))

  (component $c2
    (import "a" (type $a' (eq $a)))
    (import "f" (func $f (result $a')))
    (core func $g (canon lower (func $f)))
    (core module $m2
      (import "" "f" (func (result i32)))
      (func (export "f") (result i32) (call 0)))
    (core instance $ci2
      (instantiate $m2 (with "" (instance (export "f" (func $g))))))
    (func (export "f") (result $a') (canon lift (core func $ci2 "f"))))

  (instance $i1 (instantiate $c1 (with "a" (type $a))))
  (instance $i2 (instantiate $c2
                  (with "a" (type $a))
                  (with "f" (func $i1 "f"))))
)

;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+96
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0063                               v4 = load.i64 notrap aligned readonly can_move v0+96
;; @0063                               v5 = load.i32 notrap aligned table v4
;; @0065                               v6 = iconst.i32 1
;; @0067                               v7 = band v5, v6  ; v6 = 1
;; @0061                               v3 = iconst.i32 0
;; @0068                               v8 = icmp eq v7, v3  ; v3 = 0
;; @0068                               v9 = uextend.i32 v8
;; @006b                               trapnz v9, user11
;; @0069                               jump block3
;;
;;                                 block3:
;; @006d                               v10 = load.i64 notrap aligned readonly can_move v0+72
;; @006d                               v11 = load.i32 notrap aligned table v10
;; @006f                               v12 = iconst.i32 2
;; @0071                               v13 = band v11, v12  ; v12 = 2
;;                                     v80 = iconst.i32 0
;;                                     v81 = icmp eq v13, v80  ; v80 = 0
;; @0072                               v15 = uextend.i32 v81
;; @0075                               trapnz v15, user11
;; @0073                               jump block5
;;
;;                                 block5:
;; @0077                               v17 = load.i32 notrap aligned table v10
;; @0079                               v18 = iconst.i32 -3
;; @007b                               v19 = band v17, v18  ; v18 = -3
;; @007c                               store notrap aligned table v19, v10
;;                                     v70 = iconst.i32 -4
;;                                     v76 = band v17, v70  ; v70 = -4
;; @0083                               store notrap aligned table v76, v10
;;                                     v82 = iconst.i32 1
;;                                     v83 = bor v19, v82  ; v82 = 1
;; @008a                               store notrap aligned table v83, v10
;; @008c                               v32 = load.i64 notrap aligned readonly can_move v0+48
;; @008c                               v33 = load.i64 notrap aligned readonly can_move v0+64
;; @008c                               v34 = call_indirect sig0, v32(v33, v0)
;; @0090                               v36 = load.i32 notrap aligned table v4
;; @0080                               v23 = iconst.i32 -2
;; @0094                               v38 = band v36, v23  ; v23 = -2
;; @0095                               store notrap aligned table v38, v4
;; @009b                               v40 = iconst.i32 3
;; @009d                               v41 = icmp ugt v34, v40  ; v40 = 3
;; @009d                               v42 = uextend.i32 v41
;; @00a0                               trapnz v42, user11
;; @009e                               jump block7
;;
;;                                 block7:
;; @00a4                               v44 = load.i32 notrap aligned table v4
;;                                     v84 = iconst.i32 1
;;                                     v85 = bor v44, v84  ; v84 = 1
;; @00a9                               store notrap aligned table v85, v4
;; @00ab                               v49 = load.i32 notrap aligned table v10
;;                                     v86 = iconst.i32 2
;;                                     v87 = bor v49, v86  ; v86 = 2
;; @00b0                               store notrap aligned table v87, v10
;; @00b2                               jump block1
;;
;;                                 block1:
;; @00b2                               return v34
;; }
