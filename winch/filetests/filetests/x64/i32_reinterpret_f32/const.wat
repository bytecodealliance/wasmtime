;;! target = "x86_64"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.reinterpret_f32)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f10050c000000     	movss	xmm0, dword ptr [rip + 0xc]
;;   14:	 660f7ec0             	movd	eax, xmm0
;;   18:	 4883c408             	add	rsp, 8
;;   1c:	 5d                   	pop	rbp
;;   1d:	 c3                   	ret	
;;   1e:	 0000                 	add	byte ptr [rax], al
;;   20:	 0000                 	add	byte ptr [rax], al
