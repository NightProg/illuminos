.globl thread_switch
.globl join_thread

join_thread:
    cli
    push 0x23
    push rdi
    pushfq
    push 0x1B
    push rsi
    iretq

thread_switch:
    pop rax
    mov [rdi + 0x68], rax
    mov [rdi], r15
    mov [rdi + 0x8], r14
    mov [rdi + 0x10], r13
    mov [rdi + 0x18], r12
    mov [rdi + 0x20], r11
    mov [rdi + 0x28], r10
    mov [rdi + 0x30], r9
    mov [rdi + 0x38], r8
    mov [rdi + 0x40], rbp
    mov [rdi + 0x48], rdx
    mov [rdi + 0x50], rcx
    mov [rdi + 0x58], rbx
    mov [rdi + 0x60], rax

    pushfq
    pop rax
    mov [rdi + 0x78], rax
    mov [rdi + 0x70], rsp

    mov r15, [rsi + 0x00]
    mov r14, [rsi + 0x08]
    mov r13, [rsi + 0x10]
    mov r12, [rsi + 0x18]
    mov r11, [rsi + 0x20]
    mov r10, [rsi + 0x28]
    mov r9,  [rsi + 0x30]
    mov r8,  [rsi + 0x38]
    mov rbp, [rsi + 0x40]
    mov rdx, [rsi + 0x48]
    mov rcx, [rsi + 0x50]
    mov rbx, [rsi + 0x58]
    mov rax, [rsi + 0x60]

    mov rsp, [rsi + 0x70]
    mov rax, [rsi + 0x78]
    push rax
    popfq

    jmp [rsi + 0x68]

L:
    ret
