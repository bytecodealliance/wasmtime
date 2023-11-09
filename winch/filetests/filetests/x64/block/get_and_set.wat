;;! target = "x86_64"

(module
  (func (export "") (param i32)
    local.get 0
    block
    end
    local.set 0
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;    c:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   11:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   16:	 4153                 	push	r11
;;   18:	 58                   	pop	rax
;;   19:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1d:	 4883c410             	add	rsp, 0x10
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
