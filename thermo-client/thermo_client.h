/**
 * @file thermo_client.h
 * @author Sunip K. Mukherjee (sunipkmukherjee@gmail.com)
 * @brief Temperature and Humidity data client for PICTURE-D.
 * @version 0.0.1
 * @date 2025-06-03
 *
 * @copyright Copyright (c) 2025
 *
 */

#ifndef THERMO_CLIENT_H
#define THERMO_CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif
#include <stdint.h>

#ifndef _Nonnull
/**
 * @brief Indicate that a pointer must not be NULL.
 *
 */
#define _Nonnull
#endif

typedef struct _thermal_data_s
{
    char type;       // 'T' for temperature, 'H' for humidity
    uint32_t source; // Source sensor ID
    float value;     // Temperature in Celsius or Humidity in percentage
} thermal_data_s;

/**
 * @brief Open a serial port with the given port name, and apply necessary settings.
 *
 * @param port The name of the serial port to open (e.g., "/dev/ttyACM0").
 * @return int Positive file descriptor on success, -1 value on failure. `errno` will be set to indicate the error.
 */
int thermo_client_init(const char *_Nonnull port);

/**
 * @brief Read temperature or humidity data from the serial port.
 *
 * This function reads data in the format: "CHRIS,[T|H],uint32_t float". (space indicates no bytes in between)
 * It will block until it receives a complete data packet.
 *
 * @param fd The file descriptor of the opened serial port.
 * @param data Pointer to a thermal_data_s structure to store the read data.
 * @return int 1 on success, 0 on incomplete data, -1 value on failure. `errno` will be set to indicate the error.
 */

int thermo_client_read(int fd, thermal_data_s *_Nonnull data);

#ifdef __cplusplus
}
#endif

#endif // THERMO_CLIENT_H