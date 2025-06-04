CC = gcc
EDCFLAGS = -Wall -Wextra -pedantic -O3 -I ./ $(CFLAGS)
EDLDFLAGS = $(LDFLAGS)

all: thermo-client

SOURCES = $(wildcard *.c)
OBJECTS = $(SOURCES:.c=.o)

thermo-client: $(OBJECTS)
	$(CC) -o $@ $^ $(EDLDFLAGS)

%.o: %.c
	$(CC) $(EDCFLAGS) -c $< -o $@

clean:
	rm -f $(OBJECTS) thermo-client