;;! target = "x86_64"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.nearest)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8740000000         	ja	0x58
;;   18:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f2440f107c2408       	movsd	xmm15, qword ptr [rsp + 8]
;;      	 4883ec08             	sub	rsp, 8
;;      	 f2440f113c24         	movsd	qword ptr [rsp], xmm15
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;      	 49bb0000000000000000 	
;; 				movabs	r11, 0
;;      	 41ffd3               	call	r11
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   58:	 0f0b                 	ud2	
