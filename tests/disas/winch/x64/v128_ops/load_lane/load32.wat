;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (memory 1 1)
  (func (export "_start") (result v128)
        (v128.load32_lane
          1 (i32.const 0) (v128.const i64x2 0xFFFFFFFFFFFFFFFF 0xFFFFFFFFFFFFFFFF)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x2c(%rip), %xmm0
;;       movl    $0, %eax
;;       movq    0x50(%r14), %rcx
;;       addq    %rax, %rcx
;;       movl    (%rcx), %r11d
;;       vpinsrd $1, %r11d, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
;;   5f: addb    %bh, %bh
