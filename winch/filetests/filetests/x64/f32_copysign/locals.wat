;;! target = "x86_64"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const -1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.copysign
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f100543000000     	movss	xmm0, dword ptr [rip + 0x43]
;;   1d:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;   23:	 f30f10053d000000     	movss	xmm0, dword ptr [rip + 0x3d]
;;   2b:	 f30f11442408         	movss	dword ptr [rsp + 8], xmm0
;;   31:	 f30f10442408         	movss	xmm0, dword ptr [rsp + 8]
;;   37:	 f30f104c240c         	movss	xmm1, dword ptr [rsp + 0xc]
;;   3d:	 41bb00000080         	mov	r11d, 0x80000000
;;   43:	 66450f6efb           	movd	xmm15, r11d
;;   48:	 410f54c7             	andps	xmm0, xmm15
;;   4c:	 440f55f9             	andnps	xmm15, xmm1
;;   50:	 410f28cf             	movaps	xmm1, xmm15
;;   54:	 0f56c8               	orps	xmm1, xmm0
;;   57:	 0f28c1               	movaps	xmm0, xmm1
;;   5a:	 4883c410             	add	rsp, 0x10
;;   5e:	 5d                   	pop	rbp
;;   5f:	 c3                   	ret	
;;   60:	 cdcc                 	int	0xcc
