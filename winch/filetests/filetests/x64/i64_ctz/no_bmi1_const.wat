;;! target = "x86_64"

(module
    (func (result i64)
        (i64.const 1)
        (i64.ctz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8726000000         	ja	0x3e
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 480fbcc0             	bsf	rax, rax
;;      	 41bb00000000         	mov	r11d, 0
;;      	 410f94c3             	sete	r11b
;;      	 49c1e306             	shl	r11, 6
;;      	 4c01d8               	add	rax, r11
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3e:	 0f0b                 	ud2	
