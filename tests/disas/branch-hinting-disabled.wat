;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-Wbranch-hinting=n"

;; With branch hinting disabled, codegen ignores the `metadata.code.branch_hint`
;; custom section entirely: no block is marked `cold`.

(module
  (func $if_unlikely (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\00")
    if (result i32)
      i32.const 1
    else
      i32.const 2
    end
  )

  (func $br_if_likely (param i32) (result i32)
    (block $target
      local.get 0
      (@metadata.code.branch_hint "\01")
      br_if $target
      i32.const 1
      return
    )
    i32.const 2
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0043                               brif v2, block2, block4
;;
;;                                 block2:
;; @0045                               v5 = iconst.i32 1
;; @0047                               jump block3(v5)  ; v5 = 1
;;
;;                                 block4:
;; @0048                               v6 = iconst.i32 2
;; @004a                               jump block3(v6)  ; v6 = 2
;;
;;                                 block3(v4: i32):
;; @004b                               jump block1(v4)
;;
;;                                 block1(v3: i32):
;; @004b                               return v3
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               brif v2, block2, block3
;;
;;                                 block3:
;; @0054                               v4 = iconst.i32 1
;; @0056                               return v4  ; v4 = 1
;;
;;                                 block2:
;; @005a                               jump block1
;;
;;                                 block1:
;; @0058                               v5 = iconst.i32 2
;; @005a                               return v5  ; v5 = 2
;; }
