;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.mul)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b0424               	mov	eax, dword ptr [rsp]
;;   12:	 8b4c2404             	mov	ecx, dword ptr [rsp + 4]
;;   16:	 0fafc8               	imul	ecx, eax
;;   19:	 4889c8               	mov	rax, rcx
;;   1c:	 4883c408             	add	rsp, 8
;;   20:	 5d                   	pop	rbp
;;   21:	 c3                   	ret	
