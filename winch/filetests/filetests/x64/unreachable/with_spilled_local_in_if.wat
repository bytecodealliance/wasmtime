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
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c314000000       	add	r11, 0x14
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8732000000         	ja	0x4d
;;   1b:	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 85c0                 	test	eax, eax
;;      	 0f840f000000         	je	0x47
;;   38:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;      	 4883ec04             	sub	rsp, 4
;;      	 44891c24             	mov	dword ptr [rsp], r11d
;;      	 0f0b                 	ud2	
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4d:	 0f0b                 	ud2	
