#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

int main(int argc, char **argv) {
  if (argc < 3) {
    printf("Usage: %s <filepath> <size>\n", argv[0]);
    return 1;
  }

  int size = atoi(argv[2]) / 2;
  if (size <= 0) {
    printf("Error: Size must be a positive integer\n");
    return 1;
  }

  int *left = malloc(size * sizeof(int));
  int *right = malloc(size * sizeof(int));

  FILE *file = fopen(argv[1], "r");
  if (!file) {
    printf("Error: Could not open file %s\n", argv[1]);
    return 1;
  }
  for (size_t i = 0; i < size; i++) {
    fread(&left[i], sizeof(int), 1, file);
  }
  for (size_t i = 0; i < size; i++) {
    fread(&right[i], sizeof(int), 1, file);
  }
  fclose(file);

  double result = 0;
  for (size_t i = 0; i < size; i++) {
    result += sqrt(fabs(cos(left[i]) * sin(right[i])));
  }

  printf("%f\n", result);

  free(left);
  free(right);

  return 0;
}
