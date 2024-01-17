;;! target = "x86_64"

(module
    (func (param i32) (result f32)
        (local.get 0)
        (f32.reinterpret_i32)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 660f6ec0             	movd	xmm0, eax
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
