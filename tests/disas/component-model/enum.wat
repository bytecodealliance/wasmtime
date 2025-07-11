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
;;     fn0 = colocated u0:0 sig0
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
;;                                     v79 = iconst.i32 0
;;                                     v80 = icmp eq v13, v79  ; v79 = 0
;; @0072                               v15 = uextend.i32 v80
;; @0075                               trapnz v15, user11
;; @0073                               jump block5
;;
;;                                 block5:
;; @0077                               v17 = load.i32 notrap aligned table v10
;; @0079                               v18 = iconst.i32 -3
;; @007b                               v19 = band v17, v18  ; v18 = -3
;; @007c                               store notrap aligned table v19, v10
;;                                     v69 = iconst.i32 -4
;;                                     v75 = band v17, v69  ; v69 = -4
;; @0083                               store notrap aligned table v75, v10
;;                                     v81 = iconst.i32 1
;;                                     v82 = bor v19, v81  ; v81 = 1
;; @008a                               store notrap aligned table v82, v10
;; @008c                               v32 = load.i64 notrap aligned readonly can_move v0+64
;; @008c                               v33 = call fn0(v32, v0)
;; @0090                               v35 = load.i32 notrap aligned table v4
;; @0080                               v23 = iconst.i32 -2
;; @0094                               v37 = band v35, v23  ; v23 = -2
;; @0095                               store notrap aligned table v37, v4
;; @009b                               v39 = iconst.i32 3
;; @009d                               v40 = icmp ugt v33, v39  ; v39 = 3
;; @009d                               v41 = uextend.i32 v40
;; @00a0                               trapnz v41, user11
;; @009e                               jump block7
;;
;;                                 block7:
;; @00a4                               v43 = load.i32 notrap aligned table v4
;;                                     v83 = iconst.i32 1
;;                                     v84 = bor v43, v83  ; v83 = 1
;; @00a9                               store notrap aligned table v84, v4
;; @00ab                               v48 = load.i32 notrap aligned table v10
;;                                     v85 = iconst.i32 2
;;                                     v86 = bor v48, v85  ; v85 = 2
;; @00b0                               store notrap aligned table v86, v10
;; @00b2                               jump block1
;;
;;                                 block1:
;; @00b2                               return v33
;; }
