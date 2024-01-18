;;! target = "x86_64"

(module
    (func (result f32)
        f32.const 1.0
        i32.reinterpret_f32
        drop
        f32.const 1.0
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871e000000         	ja	0x36
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f100514000000     	movss	xmm0, dword ptr [rip + 0x14]
;;      	 660f7ec0             	movd	eax, xmm0
;;      	 f30f100508000000     	movss	xmm0, dword ptr [rip + 8]
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   36:	 0f0b                 	ud2	
;;   38:	 0000                 	add	byte ptr [rax], al
