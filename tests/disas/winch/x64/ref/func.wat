;;! target = "x86_64"
;;! test = "winch"

(module
  (elem declare func $f)
  (func $f)
  (func (export "ref-func") (result funcref)
    (ref.func $f)
  )
)
;; wasm[0]::function[0]::f:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x38
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   38: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xa1
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       leaq    0x2d(%rip), %r11
;;       movq    %r11, 0x50(%r14)
;;       leaq    -0x81(%rip), %r11
;;       movq    %r11, 0x58(%r14)
;;       movq    %r14, 0x68(%r14)
;;       movq    0x28(%r14), %r11
;;       movl    (%r11), %r11d
;;       movl    %r11d, 0x60(%r14)
;;       leaq    0x50(%r14), %rax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   a1: ud2
