;;! target = "x86_64"
;;! test = "optimize"

;; Test that branch hints from the `metadata.code.branch_hint` custom section
;; are used to mark cold blocks in the generated code.

(module
  ;; Test br_if with hint that branch is unlikely (not taken).
  ;; The branch target block should be marked cold.
  (func $unlikely_branch (param i32) (result i32)
    (block $target (result i32)
      i32.const 0      ;; value to return if branch taken
      local.get 0      ;; condition
      (@metadata.code.branch_hint "\00")
      br_if $target
      ;; Fallthrough path (likely)
      drop
      i32.const 42
    )
  )

  ;; Test br_if with hint that branch is likely (taken).
  ;; The fallthrough block should be marked cold.
  (func $likely_branch (param i32) (result i32)
    (block $target (result i32)
      i32.const 0      ;; value to return if branch taken
      local.get 0      ;; condition
      (@metadata.code.branch_hint "\01")
      br_if $target
      ;; Fallthrough path (unlikely, should be cold)
      drop
      i32.const 42
    )
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0043                               v5 = iconst.i32 0
;; @0047                               brif v2, block2(v5), block3  ; v5 = 0
;;
;;                                 block3:
;; @004a                               v6 = iconst.i32 42
;; @004c                               jump block2(v6)  ; v6 = 42
;;
;;                                 block2(v4: i32) cold:
;; @004d                               jump block1(v4)
;;
;;                                 block1(v3: i32):
;; @004d                               return v3
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0052                               v5 = iconst.i32 0
;; @0056                               brif v2, block2(v5), block3  ; v5 = 0
;;
;;                                 block3 cold:
;; @0059                               v6 = iconst.i32 42
;; @005b                               jump block2(v6)  ; v6 = 42
;;
;;                                 block2(v4: i32):
;; @005c                               jump block1(v4)
;;
;;                                 block1(v3: i32):
;; @005c                               return v3
;; }
