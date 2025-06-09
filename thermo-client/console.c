/**
 * @file console.c
 * @author your name (you@domain.com)
 * @brief 
 * @version 0.1
 * @date 2025-06-06
 * 
 * @copyright Copyright (c) 2025
 * 
 */
#include <stdio.h>
#include <unistd.h>
#include <signal.h>
#include <ncurses.h>
#include <pthread.h>
#include <string.h>
#include <errno.h>
#include "thermo_client.h"

volatile sig_atomic_t running = 1;

void sighandler(int sig)
{
    (void)sig;
    running = 0;
}

WINDOW *output_win, *input_win;
int thermo_fd = -1;
pthread_mutex_t win_lock = PTHREAD_MUTEX_INITIALIZER;

void init_ui()
{
    initscr();
    cbreak();

    int height, width;
    getmaxyx(stdscr, height, width);

    output_win = newwin(height - 3, width, 0, 0);
    scrollok(output_win, TRUE);

    input_win = newwin(3, width, height - 3, 0);
    box(input_win, 0, 0);
    mvwprintw(input_win, 1, 1, ">> ");
    wrefresh(input_win);
}

void *read_input(void *arg)
{
    (void) arg;
#define INPUT_BUF_SIZE 256
    char input_buf[INPUT_BUF_SIZE];

    while (running)
    {
        mvwgetnstr(input_win, 1, 4, input_buf, INPUT_BUF_SIZE - 1);

        if (strcmp(input_buf, "/quit") == 0)
        {
            running = 0;
            break;
        }

        if (thermo_fd && strlen(input_buf) > 0)
        {
            int w = write(thermo_fd, input_buf, strlen(input_buf));
            (void) w;
        }
        werase(input_win);
        box(input_win, 0, 0);
        mvwprintw(input_win, 1, 1, ">> ");
        wrefresh(input_win);
    }
    return NULL;
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

    init_ui();

    pthread_t input_thread;
    if (pthread_create(&input_thread, NULL, read_input, NULL) != 0)
    {
        perror("Failed to create input thread");
        endwin();
        goto end;
    }

    while (running)
    {
        char buf[1024] = {0}; // Clear the buffer
        fd = thermo_client_init(argv[1]);
        if (fd < 0)
        {
            snprintf(buf, sizeof(buf), "Error reading data: %s\n", strerror(errno));
            wprintw(output_win, "%s", buf);
            wrefresh(output_win);
            sleep(1);
            continue; // Initialization failed
        }
        thermo_fd = fd;
        while (running)
        {
            int result = thermo_client_read(fd, &data, &running);
            if (result < 0)
            {
                snprintf(buf, sizeof(buf), "Error reading data: %s\n", strerror(errno));
                break;
            }
            else if (result == 0)
            {
                continue; // No data available or malformed data, continue reading
            }
            else
            {
                snprintf(buf, sizeof(buf), "Received: Type: %c, Source: 0x%08x, Value: %.2f %c\n", data.type, data.source, data.value, data.type == 'T' ? 'C' : '%');
            }
            wprintw(output_win, "%s", buf);
            wrefresh(output_win);
        }
    }
end:
    close(fd);
    return 0;
}