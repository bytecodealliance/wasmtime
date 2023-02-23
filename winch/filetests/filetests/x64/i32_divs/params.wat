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
;;   19:	 0f8502000000         	jne	0x21
;;   1f:	 0f0b                 	ud2	
;;   21:	 83f9ff               	cmp	ecx, -1
;;   24:	 0f850e000000         	jne	0x38
;;   2a:	 81f800000080         	cmp	eax, 0x80000000
;;   30:	 0f8502000000         	jne	0x38
;;   36:	 0f0b                 	ud2	
;;   38:	 99                   	cdq	
;;   39:	 f7f9                 	idiv	ecx
;;   3b:	 4883c408             	add	rsp, 8
;;   3f:	 5d                   	pop	rbp
;;   40:	 c3                   	ret	
