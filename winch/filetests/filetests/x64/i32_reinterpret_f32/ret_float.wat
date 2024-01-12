;;! target = "x86_64"

(module
    (func (result f32)
        f32.const 1.0
        i32.reinterpret_f32
        drop
        f32.const 1.0
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f30f100514000000     	movss	xmm0, dword ptr [rip + 0x14]
;;   14:	 660f7ec0             	movd	eax, xmm0
;;   18:	 f30f100508000000     	movss	xmm0, dword ptr [rip + 8]
;;   20:	 4883c408             	add	rsp, 8
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
;;   26:	 0000                 	add	byte ptr [rax], al
;;   28:	 0000                 	add	byte ptr [rax], al
