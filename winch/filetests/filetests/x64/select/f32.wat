;;! target = "x86_64"

(module
  (func (export "select-f32") (param f32 f32 i32) (result f32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8737000000         	ja	0x4f
;;   18:	 f30f11442414         	movss	dword ptr [rsp + 0x14], xmm0
;;      	 f30f114c2410         	movss	dword ptr [rsp + 0x10], xmm1
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 f30f10442410         	movss	xmm0, dword ptr [rsp + 0x10]
;;      	 f30f104c2414         	movss	xmm1, dword ptr [rsp + 0x14]
;;      	 83f800               	cmp	eax, 0
;;      	 0f8404000000         	je	0x49
;;   45:	 f20f10c1             	movsd	xmm0, xmm1
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4f:	 0f0b                 	ud2	
