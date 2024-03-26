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
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x60, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x108
;;   1b: movq    %rdi, %r14
;;       subq    $0x50, %rsp
;;       movq    %rdi, 0x48(%rsp)
;;       movq    %rsi, 0x40(%rsp)
;;       movss   %xmm0, 0x3c(%rsp)
;;       movsd   %xmm1, 0x30(%rsp)
;;       movq    %rdx, 0x28(%rsp)
;;       movq    %rcx, 0x20(%rsp)
;;       movsd   %xmm2, 0x18(%rsp)
;;       movsd   %xmm3, 0x10(%rsp)
;;       movss   %xmm4, 0xc(%rsp)
;;       movq    %r8, (%rsp)
;;       movl    0x70(%r14), %eax
;;       cmpl    $0, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       testl   %eax, %eax
;;       je      0x72
;;   70: ud2
;;       movl    0x70(%r14), %eax
;;       subl    $1, %eax
;;       movl    %eax, 0x70(%r14)
;;       movq    0x58(%r14), %rax
;;       shrl    $0x10, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    0xc(%rsp), %esi
;;       movl    $0, %edx
;;       callq   0x2ff
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x48(%rsp), %r14
;;       movl    %eax, %eax
;;       movq    0x50(%r14), %rcx
;;       addq    %rax, %rcx
;;       addq    $0x23024, %rcx
;;       movsbq  (%rcx), %rax
;;       movss   0x47(%rip), %xmm0
;;       subq    $0xc, %rsp
;;       movsd   0x42(%rip), %xmm15
;;       movsd   %xmm15, (%rsp)
;;       movss   0x2b(%rip), %xmm15
;;       movss   %xmm15, 8(%rsp)
;;       movq    0xc(%rsp), %rax
;;       popq    %r11
;;       movq    %r11, (%rax)
;;       movl    (%rsp), %r11d
;;       addq    $4, %rsp
;;       movl    %r11d, 8(%rax)
;;       addq    $0x50, %rsp
;;       popq    %rbp
;;       retq
;;  108: ud2
;;  10a: addb    %al, (%rax)
;;  10c: addb    %al, (%rax)
;;  10e: addb    %al, (%rax)
;;  110: addb    %al, (%rax)
;;  112: addb    %al, (%rax)
;;  114: addb    %al, (%rax)
;;  116: addb    %al, (%rax)
;;  118: addb    %al, (%rax)
;;  11a: addb    %al, (%rax)
;;  11c: addb    %al, (%rax)
;;  11e: addb    %al, (%rax)
