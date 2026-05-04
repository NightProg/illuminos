#include "c/syscall.h"
#include <stdint.h>



void putstr(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    sys_write(1, s, len);  // ← écriture atomique
}

void putint(int n)
{
    char buf[12];
    int i = 0;

    if (n == 0) {
        sys_write(1, "0", 1);
        return;
    }

    while (n > 0) {
        buf[11 - i++] = '0' + (n % 10);
        n /= 10;
    }

    sys_write(1, buf + (12 - i), i);
}

void page_fault_handler()
{
    putstr("Caught a page fault signal!\n");
    sys_exit();
}


void _start(void)
{
    uint64_t pid = sys_fork();

    if (pid == 0) {
        // Child process
        putstr("Hello from the child process!: ");
        putint(pid);
        putstr("\n");
        sys_exit();
    } else if (pid == UINT64_MAX) {
        putstr("FORK FAILED\n");
        sys_exit();
    } else {
        putstr("HELLO from the parent process: ");
        putint(pid);
        putstr("\n");
        sys_exit();
    }

}
