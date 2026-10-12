;;! target = "x86_64"
;;! test = "winch"

;; `ref.func` of an imported function is a pointer to its `VMFunctionImport`
;; within the `vmctx`.

(module
  (import "env" "f" (func $f (result i32)))
  (elem declare func $f)
  (func (export "ref-func") (result funcref)
    (ref.func $f)))
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       leaq    0x30(%r14), %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3c: ud2
