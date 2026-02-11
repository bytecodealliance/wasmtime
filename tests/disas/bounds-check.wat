;;! test = "optimize"
;;! target = "x86_64"
;;! flags = ["-Omemory-reservation=0x8000000", "-Omemory-guard-size=0x100000000", "-Omemory-may-move=n"]

(module
  (memory 16)
  (func $store (param i32)
    ;; No offset. But because we have a 4 GiB guard, this needs no bounds check.
    local.get 0
    i32.const 0
    i32.store8 0

    ;; The greatest possible offset that can ever be in bounds. Again, no
    ;; bounds check.
    local.get 0
    i32.const 0
    i32.store8 0 offset=134217727

    ;; The greatest encodable offset. This will never be in bounds, given
    ;; our memory reservation size, so optimization isn't a concern.
    local.get 0
    i32.const 0
    i32.store8 0 offset=4294967295
  )
  (export "store" (func $store))
)
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move checked gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002a                               v3 = iconst.i32 0
;; @002c                               v5 = load.i64 notrap aligned readonly can_move checked v0+56
;; @002c                               v4 = uextend.i64 v2
;; @002c                               v6 = iadd v5, v4
;; @002c                               istore8 little heap v3, v6  ; v3 = 0
;; @0033                               v11 = iconst.i64 0x07ff_ffff
;; @0033                               v12 = iadd v6, v11  ; v11 = 0x07ff_ffff
;; @0033                               istore8 little heap v3, v12  ; v3 = 0
;; @003d                               v15 = load.i64 notrap aligned v0+64
;; @003d                               v16 = icmp ugt v4, v15
;; @003d                               v21 = iconst.i64 0
;; @003d                               v19 = iconst.i64 0xffff_ffff
;; @003d                               v20 = iadd v6, v19  ; v19 = 0xffff_ffff
;; @003d                               v22 = select_spectre_guard v16, v21, v20  ; v21 = 0
;; @003d                               istore8 little heap v3, v22  ; v3 = 0
;; @0044                               jump block1
;;
;;                                 block1:
;; @0044                               return
;; }
