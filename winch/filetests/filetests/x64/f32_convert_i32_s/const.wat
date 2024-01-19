;;! target = "x86_64"

(module
    (func (result f32)
        (i32.const 1)
        (f32.convert_i32_s)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8713000000         	ja	0x2b
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 f30f2ac0             	cvtsi2ss	xmm0, eax
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2b:	 0f0b                 	ud2	
