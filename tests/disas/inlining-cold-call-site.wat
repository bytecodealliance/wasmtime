;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[0]--function[2]"
;;! flags = "-Cinlining=y -Wbranch-hinting"

;; Calls in cold blocks are not inlined.

(module
  (func $hot_callee (result i32)
    (i32.const 42)
  )

  (func $cold_callee (result i32)
    (i32.const 36)
  )

  (func (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\01")
    if (result i32)
      call $hot_callee
    else
      call $cold_callee
    end
  )
)

;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     sig0 = (i64 vmctx, i64) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     fn1 = colocated u0:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @004d                               brif v2, block2, block4
;;
;;                                 block2:
;; @004f                               jump block5
;;
;;                                 block5:
;;                                     jump block6
;;
;;                                 block6:
;;                                     jump block7
;;
;;                                 block7:
;;                                     v7 = iconst.i32 42
;; @0051                               jump block3(v7)  ; v7 = 42
;;
;;                                 block4 cold:
;; @0052                               v4 = call fn1(v0, v0)
;; @0054                               jump block3(v4)
;;
;;                                 block3(v5: i32):
;; @0055                               jump block1
;;
;;                                 block1:
;; @0055                               return v5
;; }
