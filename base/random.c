#include <ctype.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

int main(int argc, char *argv[]) {
  if (argc != 3) {
    fprintf(stderr, "Usage: %s <output_file> <number_of_integers>\n", argv[0]);
    return 1;
  }

  long count = atol(argv[2]);
  if (count <= 0) {
    fprintf(stderr, "Error: Parameter must be a positive integer\n");
    return 1;
  }

  srand(time(NULL));

  FILE *file = fopen(argv[1], "w");
  if (file == NULL) {
    fprintf(stderr, "Error: Failed to open file\n");
    return 1;
  }
  for (long i = 0; i < count; i++) {
    const unsigned value = rand() % INT_MAX;
    fwrite(&value, sizeof(unsigned), 1, file);
  }
  fclose(file);

  return 0;
}
