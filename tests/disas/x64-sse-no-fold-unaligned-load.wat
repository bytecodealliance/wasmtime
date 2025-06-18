;;! target = 'x86_64'
;;! test = 'compile'
;;! flags = '-Ccranelift-has-ssse3'

(module
  (memory 1)
  (func $punpckhbw (param v128 i32) (result v128)
    local.get 0
    (v128.load offset=1 (local.get 1))
    i8x16.shuffle
      0x08 0x18 0x09 0x19
      0x0a 0x1a 0x0b 0x1b
      0x0c 0x1c 0x0d 0x1d
      0x0e 0x1e 0x0f 0x1f
    return
  )
)

;; wasm[0]::function[0]::punpckhbw:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r9d
;;       addq    0x38(%rdi), %r9
;;       movdqu  1(%r9), %xmm6
;;       punpckhbw %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
