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
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x60, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x108
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x50, %rsp
;;   22: movq    %rdi, 0x48(%rsp)
;;   27: movq    %rsi, 0x40(%rsp)
;;   2c: movss   %xmm0, 0x3c(%rsp)
;;   32: movsd   %xmm1, 0x30(%rsp)
;;   38: movq    %rdx, 0x28(%rsp)
;;   3d: movq    %rcx, 0x20(%rsp)
;;   42: movsd   %xmm2, 0x18(%rsp)
;;   48: movsd   %xmm3, 0x10(%rsp)
;;   4e: movss   %xmm4, 0xc(%rsp)
;;   54: movq    %r8, (%rsp)
;;   58: movl    0x70(%r14), %eax
;;   5c: cmpl    $0, %eax
;;   5f: movl    $0, %eax
;;   64: sete    %al
;;   68: testl   %eax, %eax
;;   6a: je      0x72
;;   70: ud2
;;   72: movl    0x70(%r14), %eax
;;   76: subl    $1, %eax
;;   79: movl    %eax, 0x70(%r14)
;;   7d: movq    0x58(%r14), %rax
;;   81: shrl    $0x10, %eax
;;   84: subq    $4, %rsp
;;   88: movl    %eax, (%rsp)
;;   8b: subq    $0xc, %rsp
;;   8f: movq    %r14, %rdi
;;   92: movl    0xc(%rsp), %esi
;;   96: movl    $0, %edx
;;   9b: callq   0x2ff
;;   a0: addq    $0xc, %rsp
;;   a4: addq    $4, %rsp
;;   a8: movq    0x48(%rsp), %r14
;;   ad: movl    %eax, %eax
;;   af: movq    0x50(%r14), %rcx
;;   b3: addq    %rax, %rcx
;;   b6: addq    $0x23024, %rcx
;;   bd: movsbq  (%rcx), %rax
;;   c1: movss   0x47(%rip), %xmm0
;;   c9: subq    $0xc, %rsp
;;   cd: movsd   0x42(%rip), %xmm15
;;   d6: movsd   %xmm15, (%rsp)
;;   dc: movss   0x2b(%rip), %xmm15
;;   e5: movss   %xmm15, 8(%rsp)
;;   ec: movq    0xc(%rsp), %rax
;;   f1: popq    %r11
;;   f3: movq    %r11, (%rax)
;;   f6: movl    (%rsp), %r11d
;;   fa: addq    $4, %rsp
;;   fe: movl    %r11d, 8(%rax)
;;  102: addq    $0x50, %rsp
;;  106: popq    %rbp
;;  107: retq
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
