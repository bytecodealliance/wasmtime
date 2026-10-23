;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-W gc-support=y -C collector=copying"

(module
  (import "" "make" (func $make (result externref)))
  (func $nop)

  ;; The second operand is a null represented internally as i32, but the
  ;; selected live reference must appear in the call-site stack map.
  (func (export "select-live") (result externref)
    call $make
    ref.null extern
    i32.const 1
    select (result externref)
    call $nop)
)
;; wasm[0]::function[1]::nop:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x38
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   38: ud2
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xcf
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movq    0x48(%r14), %rdx
;;       movq    0x38(%r14), %rcx
;;       movq    %rdx, %rdi
;;       movq    %r14, %rsi
;;       callq   *%rcx
;;       movq    8(%rsp), %r14
;;       movl    $1, %ecx
;;       movl    $0, %edx
;;       cmpl    $0, %ecx
;;       cmovnel %eax, %edx
;;       subq    $4, %rsp
;;       movl    %edx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $0xc, %rsp
;;       ╰─╼ stack_map: frame_size=32, frame_offsets=[12]
;;       movq    0xc(%rsp), %r14
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   cf: ud2
