;;! target = "x86_64"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c328000000       	add	r11, 0x28
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8741000000         	ja	0x5f
;;   1e:	 4883ec28             	sub	rsp, 0x28
;;      	 48897c2420           	mov	qword ptr [rsp + 0x20], rdi
;;      	 4889742418           	mov	qword ptr [rsp + 0x18], rsi
;;      	 f20f11442410         	movsd	qword ptr [rsp + 0x10], xmm0
;;      	 f20f114c2408         	movsd	qword ptr [rsp + 8], xmm1
;;      	 89542404             	mov	dword ptr [rsp + 4], edx
;;      	 8b442404             	mov	eax, dword ptr [rsp + 4]
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 f20f104c2410         	movsd	xmm1, qword ptr [rsp + 0x10]
;;      	 83f800               	cmp	eax, 0
;;      	 0f8404000000         	je	0x59
;;   55:	 f20f10c1             	movsd	xmm0, xmm1
;;      	 4883c428             	add	rsp, 0x28
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5f:	 0f0b                 	ud2	
