;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (memory 1 1)
  (func (export "_start")
        (v128.store8_lane
          1 (i32.const 0) (v128.const i64x2 0xFFFFFFFFFFFFFFFF 0xFFFFFFFFFFFFFFFF)
          )))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       movl    $0, %eax
;;       movq    0x50(%r14), %rcx
;;       addq    %rax, %rcx
;;       vpextrb $1, %xmm0, (%rcx)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4c: ud2
;;   4e: addb    %al, (%rax)
