#include <winsock2.h>
#include <windows.h>
#include <ws2tcpip.h>
#include <iphlpapi.h>
#include <icmpapi.h>
#include <stdio.h>
#include <stdlib.h>

#pragma comment(lib, "Iphlpapi.lib")
#pragma comment(lib, "Ws2_32.lib")

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <ip-address>\n", argv[0]);
        return 1;
    }

    const char *ip_str = argv[1];
    IPAddr ipaddr = inet_addr(ip_str);
    if (ipaddr == INADDR_NONE) {
        fprintf(stderr, "Invalid IP address: %s\n", ip_str);
        return 1;
    }

    HANDLE hIcmpFile = IcmpCreateFile();
    if (hIcmpFile == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "IcmpCreateFile failed: %lu\n", GetLastError());
        return 1;
    }

    unsigned char sendData[24] = {
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        'i', 'c', 'm', 'p', 's', 'h',
        0, 0, 0, 0, 0, 0, 0, 0, 0
    };

    DWORD replySize = sizeof(ICMP_ECHO_REPLY) + sizeof(sendData) + 8;
    void *replyBuffer = malloc(replySize);
    if (!replyBuffer) {
        fprintf(stderr, "Memory allocation failed.\n");
        IcmpCloseHandle(hIcmpFile);
        return 1;
    }

    while (1) {
        DWORD ret = IcmpSendEcho(
            hIcmpFile,
            ipaddr,
            (LPVOID)sendData,
            (WORD)sizeof(sendData),
            NULL,
            replyBuffer,
            replySize,
            1000
        );

        if (ret != 0) {
            PICMP_ECHO_REPLY echo = (PICMP_ECHO_REPLY)replyBuffer;
            struct in_addr addr;
            addr.s_addr = echo->Address;
            printf("Reply from %s: time=%lums, status=%lu\n",
                   inet_ntoa(addr),
                   echo->RoundTripTime,
                   echo->Status);
        } else {
            DWORD err = GetLastError();
            printf("Request timed out (error %lu)\n", err);
        }

        Sleep(3000);
    }

    free(replyBuffer);
    IcmpCloseHandle(hIcmpFile);
    return 0;
}
