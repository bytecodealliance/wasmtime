;;! target = "x86_64"

(module
  (func (export "") (param f32) (result f32)
    local.get 0
    block
    end
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   13:	 f3440f107c240c       	movss	xmm15, dword ptr [rsp + 0xc]
;;   1a:	 4883ec04             	sub	rsp, 4
;;   1e:	 f3440f113c24         	movss	dword ptr [rsp], xmm15
;;   24:	 f30f100424           	movss	xmm0, dword ptr [rsp]
;;   29:	 4883c404             	add	rsp, 4
;;   2d:	 4883c410             	add	rsp, 0x10
;;   31:	 5d                   	pop	rbp
;;   32:	 c3                   	ret	
