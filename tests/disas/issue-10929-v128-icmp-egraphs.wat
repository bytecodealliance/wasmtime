;;! target = 'x86_64'
;;! test = 'optimize'

(module
  (func (param v128) (result v128)
    local.get 0
    local.get 0
    local.get 0
    v128.not
    v128.xor
    i8x16.ne)
)
;; function u0:0(i64 vmctx, i64, i8x16) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     const0 = 0xffffffffffffffffffffffffffffffff
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i8x16):
;; @0025                               jump block1
;;
;;                                 block1:
;;                                     v9 = vconst.i8x16 const0
;; @0023                               v6 = icmp.i8x16 ne v2, v9  ; v9 = const0
;; @0025                               return v6
;; }
