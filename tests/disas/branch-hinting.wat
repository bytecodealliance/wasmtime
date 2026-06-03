;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-Wbranch-hinting"

;; Branch hints from the `metadata.code.branch_hint` custom section mark blocks
;; cold during compilation: `\00` = condition likely false, `\01` = likely true.
;;
;; The annotation must immediately precede the `br_if`/`if` keyword: before a
;; folded instruction the encoder attaches the hint to the first nested
;; instruction (the condition) rather than the branch.

(module
  (global $g (mut i32) (i32.const 0))

  ;; condition likely false: branch target is cold.
  (func $br_if_unlikely (param i32) (result i32)
    (block $target
      local.get 0
      (@metadata.code.branch_hint "\00")
      br_if $target
      i32.const 1
      return
    )
    i32.const 2
  )

  ;; condition likely true: fallthrough is cold.
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

  ;; condition likely false: then-block is cold.
  (func $if_unlikely (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\00")
    if (result i32)
      i32.const 1
    else
      i32.const 2
    end
  )

  ;; condition likely true: else-block is cold.
  (func $if_likely (param i32) (result i32)
    local.get 0
    (@metadata.code.branch_hint "\01")
    if (result i32)
      i32.const 1
    else
      i32.const 2
    end
  )

  ;; condition likely true with a lazily allocated else (void if/else): the
  ;; else-block is still cold (the hint is deferred to the `else`).
  (func $if_void_likely (param i32)
    local.get 0
    (@metadata.code.branch_hint "\01")
    if
      i32.const 1
      global.set $g
    else
      i32.const 2
      global.set $g
    end
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0063                               brif v2, block2, block3
;;
;;                                 block3:
;; @0065                               v4 = iconst.i32 1
;; @0067                               return v4  ; v4 = 1
;;
;;                                 block2 cold:
;; @006b                               jump block1
;;
;;                                 block1:
;; @0069                               v5 = iconst.i32 2
;; @006b                               return v5  ; v5 = 2
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0072                               brif v2, block2, block3
;;
;;                                 block3 cold:
;; @0074                               v4 = iconst.i32 1
;; @0076                               return v4  ; v4 = 1
;;
;;                                 block2:
;; @007a                               jump block1
;;
;;                                 block1:
;; @0078                               v5 = iconst.i32 2
;; @007a                               return v5  ; v5 = 2
;; }
;;
;; function u0:2(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @007f                               brif v2, block2, block4
;;
;;                                 block2 cold:
;; @0081                               v5 = iconst.i32 1
;; @0083                               jump block3(v5)  ; v5 = 1
;;
;;                                 block4:
;; @0084                               v6 = iconst.i32 2
;; @0086                               jump block3(v6)  ; v6 = 2
;;
;;                                 block3(v4: i32):
;; @0087                               jump block1(v4)
;;
;;                                 block1(v3: i32):
;; @0087                               return v3
;; }
;;
;; function u0:3(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @008c                               brif v2, block2, block4
;;
;;                                 block2:
;; @008e                               v5 = iconst.i32 1
;; @0090                               jump block3(v5)  ; v5 = 1
;;
;;                                 block4 cold:
;; @0091                               v6 = iconst.i32 2
;; @0093                               jump block3(v6)  ; v6 = 2
;;
;;                                 block3(v4: i32):
;; @0094                               jump block1(v4)
;;
;;                                 block1(v3: i32):
;; @0094                               return v3
;; }
;;
;; function u0:4(i64 vmctx, i64, i32) tail {
;;     region0 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0099                               brif v2, block2, block4
;;
;;                                 block2:
;; @009b                               v3 = iconst.i32 1
;; @009d                               store notrap aligned region0 v3, v0+48  ; v3 = 1
;; @009f                               jump block3
;;
;;                                 block4 cold:
;; @00a0                               v5 = iconst.i32 2
;; @00a2                               store notrap aligned region0 v5, v0+48  ; v5 = 2
;; @00a4                               jump block3
;;
;;                                 block3:
;; @00a5                               jump block1
;;
;;                                 block1:
;; @00a5                               return
;; }
