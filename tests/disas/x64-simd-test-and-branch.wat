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
;;       pxor    %xmm7, %xmm7
;;       pcmpeqb %xmm7, %xmm0
;;       ptest   %xmm0, %xmm0
;;       je      0x21
;;   17: movl    $0xc8, %eax
;;       jmp     0x26
;;   21: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]::i16x8.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm7, %xmm7
;;       pcmpeqw %xmm7, %xmm0
;;       ptest   %xmm0, %xmm0
;;       je      0x61
;;   57: movl    $0xc8, %eax
;;       jmp     0x66
;;   61: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[2]::i32x4.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm7, %xmm7
;;       pcmpeqd %xmm7, %xmm0
;;       ptest   %xmm0, %xmm0
;;       je      0xa1
;;   97: movl    $0xc8, %eax
;;       jmp     0xa6
;;   a1: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[3]::i64x2.all_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm7, %xmm7
;;       pcmpeqq %xmm7, %xmm0
;;       ptest   %xmm0, %xmm0
;;       je      0xe2
;;   d8: movl    $0xc8, %eax
;;       jmp     0xe7
;;   e2: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[4]::v128.any_true:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pxor    %xmm7, %xmm7
;;       pcmpeqb %xmm7, %xmm0
;;       pmovmskb %xmm0, %ecx
;;       cmpl    $0xffff, %ecx
;;       jne     0x126
;;  11c: movl    $0xc8, %eax
;;       jmp     0x12b
;;  126: movl    $0x64, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
