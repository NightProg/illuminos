#include "c/syscall.h"
#include <stdio.h>


void putstr(const char *s)
{
    while (*s) {
        sys_write(1, s, 1);
        s++;
    }
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
        buf[i++] = '0' + (n % 10);
        n /= 10;
    }

    for (int j = i - 1; j >= 0; j--) {
        sys_write(1, &buf[j], 1);
    }
}

void page_fault_handler()
{
    putstr("Caught a page fault signal!\n");
    sys_exit();
}


void _start(void)
{
    putstr("Provoking a page fault by writing to a null pointer...\n");
    sys_signal(SIG_PAGE_FAULT, page_fault_handler);
    int *ptr = NULL;
    *ptr = 42; // This will cause a page fault

    sys_exit();
}
