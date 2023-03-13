;;! target = "x86_64"

(module
    (func (param i32) (param i32) (result i32)
	(local.get 0)
	(local.get 1)
	(i32.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 897c2404             	mov	dword ptr [rsp + 4], edi
;;    c:	 893424               	mov	dword ptr [rsp], esi
;;    f:	 8b0c24               	mov	ecx, dword ptr [rsp]
;;   12:	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;   16:	 83f900               	cmp	ecx, 0
;;   19:	 0f8409000000         	je	0x28
;;   1f:	 99                   	cdq	
;;   20:	 f7f9                 	idiv	ecx
;;   22:	 4883c408             	add	rsp, 8
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	
;;   28:	 0f0b                 	ud2	
