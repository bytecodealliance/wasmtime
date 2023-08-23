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
;;   1a:	 0f840f000000         	je	0x2f
;;   20:	 e800000000           	call	0x25
;;   25:	 b803000000           	mov	eax, 3
;;   2a:	 e90a000000           	jmp	0x39
;;   2f:	 e800000000           	call	0x34
;;   34:	 b8fdffffff           	mov	eax, 0xfffffffd
;;   39:	 8b4c2408             	mov	ecx, dword ptr [rsp + 8]
;;   3d:	 50                   	push	rax
;;   3e:	 85c9                 	test	ecx, ecx
;;   40:	 0f8417000000         	je	0x5d
;;   46:	 4883ec08             	sub	rsp, 8
;;   4a:	 e800000000           	call	0x4f
;;   4f:	 4883c408             	add	rsp, 8
;;   53:	 b804000000           	mov	eax, 4
;;   58:	 e912000000           	jmp	0x6f
;;   5d:	 4883ec08             	sub	rsp, 8
;;   61:	 e800000000           	call	0x66
;;   66:	 4883c408             	add	rsp, 8
;;   6a:	 b8fbffffff           	mov	eax, 0xfffffffb
;;   6f:	 59                   	pop	rcx
;;   70:	 0fafc8               	imul	ecx, eax
;;   73:	 89c8                 	mov	eax, ecx
;;   75:	 4883c410             	add	rsp, 0x10
;;   79:	 5d                   	pop	rbp
;;   7a:	 c3                   	ret	
