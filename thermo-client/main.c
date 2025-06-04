#include <stdio.h>
#include <unistd.h>
#include "thermo_client.h"

int main(int argc, char *argv[])
{
    if (argc != 2)
    {
        fprintf(stderr, "Usage: %s <serial_port>\n", argv[0]);
        return 1;
    }
    int fd = 0;
    thermal_data_s data;
start:
    fd = thermo_client_init(argv[1]);
    if (fd < 0)
    {
        return 1; // Initialization failed
    }
    printf("Preparing to read data...\n");
    while (1)
    {
        int result = thermo_client_read(fd, &data);
        if (result < 0)
        {
            perror("Error reading data");
            goto start;
        }
        else if (result == 0)
        {
            continue; // No data available or malformed data, continue reading
        }

        printf("Received: Type: %c, Source: 0x%08x, Value: %.2f %c\n", data.type, data.source, data.value, data.type == 'T' ? 'C' : '%');
    }

    close(fd);
    return 0;
}