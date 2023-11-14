;;! target = "x86_64"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    local.tee 0
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   15:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   19:	 4883c410             	add	rsp, 0x10
;;   1d:	 5d                   	pop	rbp
;;   1e:	 c3                   	ret	
