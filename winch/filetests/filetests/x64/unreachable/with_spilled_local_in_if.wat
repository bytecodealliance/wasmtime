;;! target = "x86_64"

(module
  (func (export "")
    (local i32)
    local.get 0
    if
      local.get 0
      block
      end
      unreachable
    else
      nop
    end
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   1a:	 85c0                 	test	eax, eax
;;   1c:	 0f840d000000         	je	0x2f
;;   22:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   27:	 4153                 	push	r11
;;   29:	 0f0b                 	ud2	
;;   2b:	 4883c408             	add	rsp, 8
;;   2f:	 4883c410             	add	rsp, 0x10
;;   33:	 5d                   	pop	rbp
;;   34:	 c3                   	ret	
