;;! target = "x86_64"
;;! test = "winch"
(module
  (type (;0;) (func (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)))
  (func (;0;) (type 0) (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)
    global.get 1
    i32.eqz
    if ;; label = @1
      unreachable
    end
    global.get 1
    i32.const 1
    i32.sub
    global.set 1
    memory.size
    memory.grow
    i64.load8_s offset=143396
    (drop)
    (f32.const 0)
    (f64.const 0)
    (f32.const 0)
  )
  (memory (;1;) 10 10)
  (global (;0;) f32 f32.const 0x1.d6a0d6p+87 (;=284477330000000000000000000;))
  (global (;1;) (mut i32) i32.const 1000)
  (export "main" (func 0))
  (export "0" (memory 0))
  (export "1" (global 0))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rsi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x70, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x120
;;   1b: movq    %rsi, %r14
;;       subq    $0x60, %rsp
;;       movq    %rsi, 0x58(%rsp)
;;       movq    %rdx, 0x50(%rsp)
;;       movss   %xmm0, 0x4c(%rsp)
;;       movsd   %xmm1, 0x40(%rsp)
;;       movq    %rcx, 0x38(%rsp)
;;       movq    %r8, 0x30(%rsp)
;;       movsd   %xmm2, 0x28(%rsp)
;;       movsd   %xmm3, 0x20(%rsp)
;;       movss   %xmm4, 0x1c(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movl    0x80(%r14), %eax
;;       cmpl    $0, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       testl   %eax, %eax
;;       je      0x76
;;   74: ud2
;;       movl    0x80(%r14), %eax
;;       subl    $1, %eax
;;       movl    %eax, 0x80(%r14)
;;       movq    0x68(%r14), %rax
;;       shrl    $0x10, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       movl    $0, %edx
;;       callq   0x2e9
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x58(%rsp), %r14
;;       movl    %eax, %eax
;;       movq    0x60(%r14), %rcx
;;       addq    %rax, %rcx
;;       addq    $0x23024, %rcx
;;       movsbq  (%rcx), %rax
;;       movss   0x55(%rip), %xmm0
;;       subq    $0xc, %rsp
;;       movsd   0x50(%rip), %xmm15
;;       movsd   %xmm15, (%rsp)
;;       movss   0x39(%rip), %xmm15
;;       movss   %xmm15, 8(%rsp)
;;       movq    0x14(%rsp), %rax
;;       movsd   (%rsp), %xmm15
;;       addq    $8, %rsp
;;       movsd   %xmm15, (%rax)
;;       movss   (%rsp), %xmm15
;;       addq    $4, %rsp
;;       movss   %xmm15, 8(%rax)
;;       addq    $0x60, %rsp
;;       popq    %rbp
;;       retq
;;  120: ud2
;;  122: addb    %al, (%rax)
;;  124: addb    %al, (%rax)
;;  126: addb    %al, (%rax)
;;  128: addb    %al, (%rax)
;;  12a: addb    %al, (%rax)
;;  12c: addb    %al, (%rax)
;;  12e: addb    %al, (%rax)
;;  130: addb    %al, (%rax)
;;  132: addb    %al, (%rax)
;;  134: addb    %al, (%rax)
;;  136: addb    %al, (%rax)
