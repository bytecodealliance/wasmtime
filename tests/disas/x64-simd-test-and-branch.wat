;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Ccranelift-sse41"]

(module
  (func $i8x16.all_true (param v128) (result i32)
    local.get 0
    i8x16.all_true
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )

  (func $i16x8.all_true (param v128) (result i32)
    local.get 0
    i16x8.all_true
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )

  (func $i32x4.all_true (param v128) (result i32)
    local.get 0
    i32x4.all_true
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )

  (func $i64x2.all_true (param v128) (result i32)
    local.get 0
    i64x2.all_true
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )

  (func $v128.any_true (param v128) (result i32)
    local.get 0
    v128.any_true
    if (result i32)
      i32.const 100
    else
      i32.const 200
    end
  )
)

;; wasm[0]::function[0]::i8x16.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm1, %xmm1
;;       pcmpeqb %xmm1, %xmm0
;;       ptest   %xmm0, %xmm0
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       testl   %eax, %eax
;;       jne     0x2b
;;   21: movl    $0xc8, %eax
;;       jmp     0x30
;;   2b: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::i16x8.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm1, %xmm1
;;       pcmpeqw %xmm1, %xmm0
;;       ptest   %xmm0, %xmm0
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       testl   %eax, %eax
;;       jne     0x6b
;;   61: movl    $0xc8, %eax
;;       jmp     0x70
;;   6b: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::i32x4.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm1, %xmm1
;;       pcmpeqd %xmm1, %xmm0
;;       ptest   %xmm0, %xmm0
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       testl   %eax, %eax
;;       jne     0xab
;;   a1: movl    $0xc8, %eax
;;       jmp     0xb0
;;   ab: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::i64x2.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm1, %xmm1
;;       pcmpeqq %xmm1, %xmm0
;;       ptest   %xmm0, %xmm0
;;       sete    %r8b
;;       movzbl  %r8b, %eax
;;       testl   %eax, %eax
;;       jne     0xec
;;   e2: movl    $0xc8, %eax
;;       jmp     0xf1
;;   ec: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::v128.any_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       ptest   %xmm0, %xmm0
;;       setne   %r11b
;;       movzbl  %r11b, %r11d
;;       testl   %r11d, %r11d
;;       jne     0x124
;;  11a: movl    $0xc8, %eax
;;       jmp     0x129
;;  124: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
