;;! target = "x86_64"

(module
  ;; Imported and immutable: should become a vmctx load.
  (import "env" "a" (global $imp-imm i32))

  ;; Imported and mutable: should become a vmctx load.
  (import "env" "b" (global $imp-mut (mut i32)))

  ;; Defined and immutable: should become a constant.
  (global $def-imm i32 (i32.const 42))

  ;; Defined and mutable: should become a vmctx load.
  (global $def-mut (mut i32) (i32.const 36))

  (func $f0 (result i32)
    (global.get $imp-imm)
  )

  (func $f1 (result i32)
    (global.get $imp-mut)
  )

  (func $f2 (result i32)
    (global.get $def-imm)
  )

  (func $f3 (result i32)
    (global.get $def-mut)
  )
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+48
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @003d                               v3 = load.i64 notrap aligned readonly can_move v0+48
;; @003d                               v4 = load.i32 notrap aligned table v3
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return v4
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+72
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0042                               v3 = load.i64 notrap aligned readonly can_move v0+72
;; @0042                               v4 = load.i32 notrap aligned table v3
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return v4
;; }
;;
;; function u0:2(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0047                               v3 = iconst.i32 42
;; @0049                               jump block1
;;
;;                                 block1:
;; @0049                               return v3  ; v3 = 42
;; }
;;
;; function u0:3(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @004c                               v4 = load.i32 notrap aligned table v0+112
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return v4
;; }
