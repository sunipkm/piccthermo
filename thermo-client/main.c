#include <stdio.h>
#include <unistd.h>
#include <signal.h>
#include "thermo_client.h"

volatile sig_atomic_t running = 1;

void sighandler(int sig)
{
    (void) sig;
    running = 0;
}

int main(int argc, char *argv[])
{
    if (argc != 2)
    {
        fprintf(stderr, "Usage: %s <serial_port>\n", argv[0]);
        return 1;
    }
    int fd = 0;
    thermal_data_s data;
    signal(SIGINT, sighandler);
    while (running)
    {
        fd = thermo_client_init(argv[1]);
        if (fd < 0)
        {
            sleep(1);
            continue; // Initialization failed
        }
        printf("Preparing to read data...\n");
        while (running)
        {
            int result = thermo_client_read(fd, &data, &running);
            if (result < 0)
            {
                perror("Error reading data");
                break;
            }
            else if (result == 0)
            {
                continue; // No data available or malformed data, continue reading
            }

            printf("Received: Type: %c, Source: 0x%08x, Value: %.2f %c\n", data.type, data.source, data.value, data.type == 'T' ? 'C' : '%');
        }
    }
    close(fd);
    return 0;
}