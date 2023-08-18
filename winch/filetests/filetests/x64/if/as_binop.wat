;;! target = "x86_64"

(module
  (func $dummy)
  (func (export "as-binary-operand") (param i32 i32) (result i32)
    (i32.mul
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 3))
        (else (call $dummy) (i32.const -3))
      )
      (if (result i32) (local.get 1)
        (then (call $dummy) (i32.const 4))
        (else (call $dummy) (i32.const -5))
      )
    )
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883c408             	add	rsp, 8
;;   10:	 5d                   	pop	rbp
;;   11:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 89742408             	mov	dword ptr [rsp + 8], esi
;;   10:	 4c893424             	mov	qword ptr [rsp], r14
;;   14:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   18:	 85c0                 	test	eax, eax
;;   1a:	 0f8411000000         	je	0x31
;;   20:	 e800000000           	call	0x25
;;   25:	 48c7c003000000       	mov	rax, 3
;;   2c:	 e90f000000           	jmp	0x40
;;   31:	 e800000000           	call	0x36
;;   36:	 48b8fdffffff00000000 	
;; 				movabs	rax, 0xfffffffd
;;   40:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   44:	 50                   	push	rax
;;   45:	 85c9                 	test	ecx, ecx
;;   47:	 0f8419000000         	je	0x66
;;   4d:	 4883ec08             	sub	rsp, 8
;;   51:	 e800000000           	call	0x56
;;   56:	 4883c408             	add	rsp, 8
;;   5a:	 48c7c004000000       	mov	rax, 4
;;   61:	 e917000000           	jmp	0x7d
;;   66:	 4883ec08             	sub	rsp, 8
;;   6a:	 e800000000           	call	0x6f
;;   6f:	 4883c408             	add	rsp, 8
;;   73:	 48b8fbffffff00000000 	
;; 				movabs	rax, 0xfffffffb
;;   7d:	 59                   	pop	rcx
;;   7e:	 0fafc8               	imul	ecx, eax
;;   81:	 4889c8               	mov	rax, rcx
;;   84:	 4883c410             	add	rsp, 0x10
;;   88:	 5d                   	pop	rbp
;;   89:	 c3                   	ret	
