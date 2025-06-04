#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <termios.h>

#include "thermo_client.h"

int thermo_client_init(const char *port)
{
    int fd = open(port, O_RDWR);
    if (fd == -1)
    {
        perror("Error opening port");
        return -1;
    }

    struct termios options;
    if (tcgetattr(fd, &options) < 0)
    {
        perror("Error getting terminal attributes");
        close(fd);
        return -1;
    }
    cfsetospeed(&options, B115200); // Set output baud rate
    cfsetispeed(&options, B115200); // Set input baud rate

    options.c_cflag &= ~PARENB; // No parity
    options.c_cflag &= ~CSTOPB; // One stop bit
    options.c_cflag &= ~CSIZE;
    options.c_cflag |= CS8;            // 8 data bits
    options.c_cflag |= CREAD | CLOCAL; // Enable receiver, ignore modem control lines

    options.c_lflag &= ~ICANON; // Set raw mode
    options.c_lflag &= ~(ECHO | ECHOE | ISIG);

    options.c_iflag &= ~(IXON | IXOFF | IXANY); // Disable flow control
    options.c_iflag &= ~(ICRNL | INLCR | IGNCR);

    options.c_oflag &= ~OPOST; // Disable output processing

    options.c_cc[VMIN] = 0;
    options.c_cc[VTIME] = 1; // Set timeout to 100 milliseconds (1 deciseconds)
    tcflush(fd, TCIFLUSH);
    if (tcsetattr(fd, TCSANOW, &options) < 0)
    {
        perror("Error setting terminal attributes");
        close(fd);
        return -1;
    }
    tcflush(fd, TCIFLUSH);
    return fd;
}

int thermo_client_read(int fd, thermal_data_s *data)
{
    if (fd < 0 || data == NULL)
    {
        fprintf(stderr, "Invalid file descriptor or data pointer\n");
        return -1;
    }
    static char pattern[] = "CHRIS,";
    static int pattern_length = sizeof(pattern) / sizeof(pattern[0]) - 1; // Exclude null terminator
    char check;
    // We are looking for a data of the format: CHRIS,[T|H],uint32_t,float (6 + 1 + 1 + 4 + 4 = 16 bytes)
    ssize_t bytes_read = 0;
    int index = 0;
    while (1)
    {
        bytes_read = read(fd, &check, sizeof(check));
        if (bytes_read < 0)
        {
            return bytes_read; // Error reading from the file descriptor
        }
        else if (bytes_read == 0)
        {
            // No data available, check for signs of life
            // This is a hack, we should switch to poll(1) if
            // any client-server communication is establised.
            bytes_read = write(fd, &check, sizeof(check));
            if (bytes_read < 0)
            {
                return bytes_read;
            }
            continue;
        }
        // printf("Received: %c", check);
        if (check == pattern[index]) // is it the start of a valid message?
        {
            index++;
        }
        else
        {
            index = 0; // Reset index if the character does not match
        }
        if (index == pattern_length) // If we have matched the entire pattern
        {
            // Read the next 11 bytes for type, source, and value
            uint8_t buffer[sizeof(data->type) + 1 + sizeof(data->source) + sizeof(data->value)]; // 1 byte for type, 1 byte for comma, 4 bytes for source, 4 bytes for value

            bytes_read = read(fd, buffer, sizeof(buffer));
            if (bytes_read < 0)
            {
                return -1; // Error or no data
            }
            else if (bytes_read < (ssize_t)sizeof(buffer) || buffer[1] != ',') // Check if we have enough data and the second byte is a comma
            {
                return 0; // Incomplete data
            }
            // Now we have a complete message, parse it
            data->type = buffer[0];                                    // First byte is type
            memcpy(&(data->source), buffer + 2, sizeof(data->source)); // Next byte is comma, then 4 bytes for source
            memcpy(&(data->value), buffer + 6, sizeof(data->value));   // Last 4 bytes for value

            break; // Exit the loop after reading a complete message
        }
    }

    return 1; // Success
}