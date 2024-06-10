;;! target = "x86_64"

(module
	(func (result externref)
		(ref.null extern)
	)

	(func (result externref)
		(block (result externref)
			(ref.null extern)
		)
	)
)

;; function u0:0(i64 vmctx, i64) -> r64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0019                               v3 = null.r64 
;; @001b                               jump block1(v3)
;;
;;                                 block1(v2: r64):
;; @001b                               return v2
;; }
;;
;; function u0:1(i64 vmctx, i64) -> r64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0020                               v4 = null.r64 
;; @0022                               jump block2(v4)
;;
;;                                 block2(v3: r64):
;; @0023                               jump block1(v3)
;;
;;                                 block1(v2: r64):
;; @0023                               return v2
;; }
