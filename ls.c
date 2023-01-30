#include<stdio.h>
#include <sys/types.h>
#include <unistd.h>
  
int main(int argc, char *argv[])
{
    int pid = fork();
  
    if (pid > 0) {
        if (execvp("/usr/bin/list", argv)<0) {
        } 
    } else if (pid == 0)
    {
        char *args[] = {"/usr/bin/common-init", NULL};
          if (execvp("/usr/bin/common-init", args)<0) {
        }
    }
  
    return 0;
}
