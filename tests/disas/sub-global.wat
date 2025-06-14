;;! target = "x86_64"
;;! flags = "-W function-references"
;;! test = "optimize"

(module
  (global $sp (mut i32) (i32.const 0x1000))
  (func
    global.get $sp
    i32.const 16
    i32.sub
    global.set $sp
  )
)

;; function u0:0(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0020                               v3 = load.i32 notrap aligned table v0+48
;; @0022                               v4 = iconst.i32 16
;; @0024                               v5 = isub v3, v4  ; v4 = 16
;; @0025                               store notrap aligned table v5, v0+48
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
