;;! target = "x86_64"

(module
    (func (param f32) (result i64)
        (local.get 0)
        (i64.trunc_f32_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   18:	 f3480f2cc0           	cvttss2si	rax, xmm0
;;   1d:	 4883f801             	cmp	rax, 1
;;   21:	 0f812d000000         	jno	0x54
;;   27:	 0f2ec0               	ucomiss	xmm0, xmm0
;;   2a:	 0f8a2a000000         	jp	0x5a
;;   30:	 41bb000000df         	mov	r11d, 0xdf000000
;;   36:	 66450f6efb           	movd	xmm15, r11d
;;   3b:	 410f2ec7             	ucomiss	xmm0, xmm15
;;   3f:	 0f8217000000         	jb	0x5c
;;   45:	 66450f57ff           	xorpd	xmm15, xmm15
;;   4a:	 440f2ef8             	ucomiss	xmm15, xmm0
;;   4e:	 0f820a000000         	jb	0x5e
;;   54:	 4883c410             	add	rsp, 0x10
;;   58:	 5d                   	pop	rbp
;;   59:	 c3                   	ret	
;;   5a:	 0f0b                 	ud2	
;;   5c:	 0f0b                 	ud2	
;;   5e:	 0f0b                 	ud2	
