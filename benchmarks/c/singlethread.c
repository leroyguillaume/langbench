#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>

int main(int argc, char **argv) {
  if (argc < 3) {
    fprintf(stderr, "Usage: %s <filepath> <size>\n", argv[0]);
    return 1;
  }

  int size = atoi(argv[2]);
  if (size <= 0) {
    fprintf(stderr, "Error: Size must be a positive integer\n");
    return 1;
  }
  int half_size = size / 2;

  int fd = open(argv[1], O_RDONLY);
  if (fd == -1) {
    fprintf(stderr, "Error: Could not open file %s\n", argv[1]);
    return 1;
  }

  int *data = mmap(NULL, size * sizeof(int), PROT_READ, MAP_PRIVATE, fd, 0);
  if (data == MAP_FAILED) {
    fprintf(stderr, "Error: Memory mapping failed\n");
    close(fd);
    return 1;
  }

  double result = 0;
  for (size_t i = 0; i < half_size; i++) {
    result += sqrt(fabs(cos(data[i]) * sin(data[half_size + i])));
  }

  printf("%f\n", result);

  munmap(data, size * sizeof(int));
  close(fd);

  return 0;
}
