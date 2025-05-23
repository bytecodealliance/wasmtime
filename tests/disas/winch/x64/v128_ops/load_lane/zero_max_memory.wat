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
;;       ja      0x67
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $0, %eax
;;       testl   %eax, %eax
;;       je      0x3e
;;   3c: ud2
;;       movdqu  0x2a(%rip), %xmm0
;;       movl    $0, %eax
;;       movq    8(%r14), %rcx
;;       movq    (%rcx), %r11
;;       addq    $1, %r11
;;       movq    %r11, (%rcx)
;;       ud2
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   67: ud2
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, (%rax)
;;   73: addb    %al, (%rax)
;;   75: addb    %al, (%rax)
;;   77: addb    %al, (%rax)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %al, (%rax)
