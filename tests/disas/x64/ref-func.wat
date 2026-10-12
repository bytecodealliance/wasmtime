;;! target = "x86_64"
;;! test = "compile"

;; `ref.func` of an imported function is a pointer to its `VMFunctionImport`
;; within the `vmctx`.

(module
  (import "env" "f" (func $imported (result i32)))
  (func $defined (result i32) i32.const 42)
  (func (export "ref-func-imported") (result funcref)
    (ref.func $imported))
  (func (export "ref-func-defined") (result funcref)
    (ref.func $defined))

  (elem declare func $imported $defined)
)
;; wasm[0]::function[1]::defined:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    $0x2a, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       leaq    0x30(%rdi), %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       leaq    0x36(%rip), %rsi
;;       movq    %rsi, 0xb0(%rdi)
;;       leaq    -0x59(%rip), %r8
;;       movq    %r8, 0xb8(%rdi)
;;       movq    %rdi, 0xc8(%rdi)
;;       movq    0x28(%rdi), %r9
;;       movl    (%r9), %r9d
;;       movl    %r9d, 0xc0(%rdi)
;;       leaq    0xb0(%rdi), %rax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
