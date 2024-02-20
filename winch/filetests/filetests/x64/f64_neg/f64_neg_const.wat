;;! target = "x86_64"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.neg)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c308000000       	add	r11, 8
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f872a000000         	ja	0x45
;;   1b:	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f20f10051d000000     	movsd	xmm0, qword ptr [rip + 0x1d]
;;      	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;      	 664d0f6efb           	movq	xmm15, r11
;;      	 66410f57c7           	xorpd	xmm0, xmm15
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   45:	 0f0b                 	ud2	
;;   47:	 001f                 	add	byte ptr [rdi], bl
;;   49:	 85eb                 	test	ebx, ebp
;;   4b:	 51                   	push	rcx
