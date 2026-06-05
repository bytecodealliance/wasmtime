;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (global $x (mut i31ref) (ref.i31 (i32.const 42)))
  (func (export "get") (result i31ref)
    (global.get $x)
  )
  (func (export "set") (param i31ref)
    (global.set $x (local.get 0))
  )
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0036                               v4 = iconst.i64 48
;; @0036                               v5 = iadd v0, v4  ; v4 = 48
;; @0036                               v6 = load.i32 notrap aligned v5
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return v6
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003d                               v4 = iconst.i64 48
;; @003d                               v5 = iadd v0, v4  ; v4 = 48
;; @003d                               store notrap aligned v2, v5
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return
;; }
