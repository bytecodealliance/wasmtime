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
;;   3d:	 4883ec04             	sub	rsp, 4
;;   41:	 890424               	mov	dword ptr [rsp], eax
;;   44:	 85c9                 	test	ecx, ecx
;;   46:	 0f8417000000         	je	0x63
;;   4c:	 4883ec0c             	sub	rsp, 0xc
;;   50:	 e800000000           	call	0x55
;;   55:	 4883c40c             	add	rsp, 0xc
;;   59:	 b804000000           	mov	eax, 4
;;   5e:	 e912000000           	jmp	0x75
;;   63:	 4883ec0c             	sub	rsp, 0xc
;;   67:	 e800000000           	call	0x6c
;;   6c:	 4883c40c             	add	rsp, 0xc
;;   70:	 b8fbffffff           	mov	eax, 0xfffffffb
;;   75:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   78:	 4883c404             	add	rsp, 4
;;   7c:	 0fafc8               	imul	ecx, eax
;;   7f:	 89c8                 	mov	eax, ecx
;;   81:	 4883c410             	add	rsp, 0x10
;;   85:	 5d                   	pop	rbp
;;   86:	 c3                   	ret	
