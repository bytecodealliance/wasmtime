;;! target = "x86_64"

(module
    (func (result i32)
        i32.const 1
        f32.reinterpret_i32
        drop
        i32.const 1
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8718000000         	ja	0x30
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 660f6ec0             	movd	xmm0, eax
;;      	 b801000000           	mov	eax, 1
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   30:	 0f0b                 	ud2	
