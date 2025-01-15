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

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0019                               v3 = iconst.i32 0
;; @001b                               jump block1
;;
;;                                 block1:
;; @001b                               return v3  ; v3 = 0
;; }
;;
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0020                               v4 = iconst.i32 0
;; @0022                               jump block2
;;
;;                                 block2:
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v4  ; v4 = 0
;; }
