CC = gcc
EDCFLAGS = -Wall -Wextra -pedantic -O3 -I ./ $(CFLAGS)
EDLDFLAGS = -lpthread -lncurses $(LDFLAGS)

all: thermo-client

SOURCES = $(wildcard thermo_*.c)
OBJECTS = $(SOURCES:.c=.o)

all: thermo-client thermo-console

thermo-client: main.c $(OBJECTS)
	$(CC) -o $@ $^ $(EDLDFLAGS)

thermo-console: console.c $(OBJECTS)
	$(CC) -o $@ $^ $(EDLDFLAGS)

%.o: %.c
	$(CC) $(EDCFLAGS) -c $< -o $@

clean:
	rm -f $(OBJECTS) thermo-client thermo-console