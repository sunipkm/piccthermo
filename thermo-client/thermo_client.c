/**
 * @file thermo_client.h
 * @author Sunip K. Mukherjee (sunipkmukherjee@gmail.com)
 * @brief Temperature and Humidity data client for PICTURE-D: Implementations for reading data from a serial port.
 * @version 0.0.1
 * @date 2025-06-03
 *
 * @copyright Copyright (c) 2025
 *
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <termios.h>
#include <signal.h>
#include <poll.h>

#include "thermo_client.h"

int thermo_client_init(const char *port, thermal_port_s *desc)
{
    int fd = open(port, O_RDWR);
    if (fd == -1)
    {
        return -1;
    }

    struct termios options;
    if (tcgetattr(fd, &options) < 0)
    {
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
        close(fd);
        return -1;
    }
    tcflush(fd, TCIFLUSH);
    desc->fd = fd;
    desc->synced = 0;
    return fd;
}

int thermo_client_read(thermal_port_s *desc, thermal_data_s *data, volatile sig_atomic_t *running)
{
    int fd = desc->fd;
    if (fd < 0 || data == NULL)
    {
        fprintf(stderr, "Invalid file descriptor or data pointer\n");
        return -1;
    }
    static char pattern[] = "CHRIS,";
    static int pattern_length = sizeof(pattern) / sizeof(pattern[0]) - 1; // Exclude null terminator
    char check;
    // We are looking for a data of the format: CHRIS,[T|H],uint32_t float (6 + 1 + 1 + 4 + 4 = 16 bytes)
    ssize_t bytes_read = 0;
    int index = 0;
    struct pollfd pfd;
    pfd.fd = fd;
    pfd.events = POLLIN | POLLERR | POLLHUP; // Monitor for input, errors, and hangups

    while (*running)
    {
        // Use poll to wait for data or timeout
        int poll_result = poll(&pfd, 1, 100); // Wait for 100 milliseconds
        if (poll_result < 0)
        {
            return -1; // Error occurred during polling
        }
        else if (poll_result == 0)
        {
            // Timeout occurred, continue to check for data
            continue;
        }
        if (pfd.revents & (POLLERR | POLLHUP))
        {
            // An error or hangup occurred, return -1
            return -1;
        }
        if (!desc->synced)
        {
            bytes_read = read(fd, &check, sizeof(check));
            if (bytes_read < 0)
            {
                return bytes_read; // Error reading from the file descriptor
            }
            else if (bytes_read == 0)
            {
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
                
                desc->synced = 1;                                          // indicate we are synchronized

                break; // Exit the loop after reading a complete message
            }
        }
        else if (desc->synced == 1)
        {
            uint8_t buffer[16] = {0, };
            bytes_read = 0;
            while (bytes_read < (ssize_t) sizeof(buffer) && *running)
            {
                ssize_t n = read(fd, buffer + bytes_read, sizeof(buffer) - bytes_read);
                if (n < 0)
                {
                    desc->synced = 0;
                    return n;
                }
                bytes_read += n;
            }
            for (ssize_t i = 0; i < (ssize_t) strlen(pattern); i++)
            {
                if (buffer[i] != pattern[i])
                {
                    desc->synced = 0;
                    return -1;
                }
                // Now we have a complete message, parse it
                size_t ofst = strlen(pattern);
                data->type = buffer[ofst];                                    // First byte is type
                memcpy(&(data->source), buffer + ofst + 2, sizeof(data->source)); // Next byte is comma, then 4 bytes for source
                memcpy(&(data->value), buffer + ofst + 6, sizeof(data->value));   // Last 4 bytes for value
            }
        }
        else
        {
            return -1; // Error: Invalid boolean
        }
    }

    return 1; // Success
}