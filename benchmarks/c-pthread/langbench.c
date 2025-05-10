#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef struct {
  int *left;
  int *right;
  size_t size;
  double result;
} Chunk;

void *compute(void *args) {
  Chunk *chunk = (Chunk *)args;
  for (size_t i = 0; i < chunk->size; i++) {
    chunk->result += sqrt(fabs(cos(chunk->left[i]) * sin(chunk->right[i])));
  }
  return NULL;
}

int main(int argc, char **argv) {
  if (argc < 4) {
    fprintf(stderr, "Usage: %s <filepath> <size> <threads>\n", argv[0]);
    return 1;
  }

  int size = atoi(argv[2]) / 2;
  if (size <= 0) {
    fprintf(stderr, "Error: Size must be a positive integer\n");
    return 1;
  }

  int num_threads = atoi(argv[3]);
  if (num_threads <= 0) {
    fprintf(stderr, "Error: Threads must be a positive integer\n");
    return 1;
  }

  int *left = malloc(size * sizeof(int));
  int *right = malloc(size * sizeof(int));

  FILE *file = fopen(argv[1], "r");
  if (!file) {
    fprintf(stderr, "Error: Could not open file %s\n", argv[1]);
    return 1;
  }
  fread(left, sizeof(int), size, file);
  fread(right, sizeof(int), size, file);
  fclose(file);

  unsigned chunk_size = size / num_threads;
  unsigned chunk_size_overflow = size % num_threads;

  pthread_t threads[num_threads];
  Chunk chunks[num_threads];
  for (size_t i = 0; i < num_threads; i++) {
    chunks[i].size = chunk_size + chunk_size_overflow;
    chunks[i].left = left + chunk_size * i;
    chunks[i].right = right + chunk_size * i;
    chunks[i].result = 0;
    pthread_create(&threads[i], NULL, compute, &chunks[i]);
    --chunk_size_overflow;
  }

  double result = 0;
  for (size_t i = 0; i < num_threads; i++) {
    pthread_join(threads[i], NULL);
    result += chunks[i].result;
  }

  printf("%f\n", result);

  free(left);
  free(right);

  return 0;
}
