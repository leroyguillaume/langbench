#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>

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

  int size = atoi(argv[2]);
  if (size <= 0) {
    fprintf(stderr, "Error: Size must be a positive integer\n");
    return 1;
  }
  int half_size = size / 2;

  int num_threads = atoi(argv[3]);
  if (num_threads <= 0) {
    fprintf(stderr, "Error: Threads must be a positive integer\n");
    return 1;
  }

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

  unsigned chunk_size = half_size / num_threads;
  unsigned chunk_size_overflow = half_size % num_threads;

  pthread_t threads[num_threads];
  Chunk chunks[num_threads];
  size_t current_pos = 0;
  for (size_t i = 0; i < num_threads; i++) {
    chunks[i].size = chunk_size + (i < chunk_size_overflow ? 1 : 0);
    chunks[i].left = data + current_pos;
    chunks[i].right = data + half_size + current_pos;
    chunks[i].result = 0;
    current_pos += chunks[i].size;
    pthread_create(&threads[i], NULL, compute, &chunks[i]);
  }

  double result = 0;
  for (size_t i = 0; i < num_threads; i++) {
    pthread_join(threads[i], NULL);
    result += chunks[i].result;
  }

  printf("%f\n", result);

  munmap(data, size * sizeof(int));
  close(fd);

  return 0;
}
