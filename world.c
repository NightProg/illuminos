#include "c/syscall.h"


int main()
{
    sys_write(1, "Executing /mnt/hello\n", 22);
    char* argv[] = {"/mnt/hello", "arg1", "arg2", NULL};
    sys_execve("/mnt/hello", argv, NULL);
}
