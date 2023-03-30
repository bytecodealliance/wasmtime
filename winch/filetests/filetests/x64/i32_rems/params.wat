;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   12:	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;   16:	 99                   	cdq	
;;   17:	 83f9ff               	cmp	ecx, -1
;;   1a:	 0f850a000000         	jne	0x2a
;;   20:	 ba00000000           	mov	edx, 0
;;   25:	 e902000000           	jmp	0x2c
;;   2a:	 f7f9                 	idiv	ecx
;;   2c:	 4889d0               	mov	rax, rdx
;;   2f:	 4883c408             	add	rsp, 8
;;   33:	 5d                   	pop	rbp
;;   34:	 c3                   	ret	
