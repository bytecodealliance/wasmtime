;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
  (memory 0 0)
  (func (result f32)
    i32.const 0
    if
      unreachable
    end
    i32.const 0
    v128.const i64x2 0 0
    v128.load64_lane align=1 0
    drop
    f32.const 0
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %eax
;;       testl   %eax, %eax
;;       je      0x3b
;;   39: ud2
;;       movdqu  0x1d(%rip), %xmm0
;;       movl    $0, %eax
;;       movq    8(%r14), %rcx
;;       movq    (%rcx), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rcx)
;;       ud2
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5e: ud2
;;   60: addb    %al, (%rax)
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
;;   66: addb    %al, (%rax)
;;   68: addb    %al, (%rax)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
