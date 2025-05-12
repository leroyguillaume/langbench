#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>

#ifdef __x86_64__
#include <immintrin.h>  // For AVX2 instructions
#endif

#define CACHE_LINE_SIZE 64
#define ALIGN_UP(x, align) (((x) + ((align)-1)) & ~((align)-1))
#define BLOCK_SIZE 64  // Process data in blocks that fit in L1 cache

typedef struct {
  int *left;
  int *right;
  size_t size;
  double result;
  char pad[CACHE_LINE_SIZE - sizeof(double)];  // Prevent false sharing
} Chunk __attribute__((aligned(CACHE_LINE_SIZE)));

#ifdef __x86_64__
// Process 8 elements at once using AVX2
static inline double process_chunk_avx2(const int *left, const int *right, size_t size) {
    double result = 0.0;
    size_t i;

    // Process 8 elements at a time using AVX2
    for (i = 0; i + 8 <= size; i += 8) {
        __m256d sum = _mm256_setzero_pd();

        for (int j = 0; j < 8; j++) {
            double val = sqrt(fabs(cos(left[i + j]) * sin(right[i + j])));
            sum = _mm256_add_pd(sum, _mm256_set1_pd(val));
        }

        double temp[4];
        _mm256_storeu_pd(temp, sum);
        result += temp[0] + temp[1] + temp[2] + temp[3];
    }

    // Handle remaining elements
    for (; i < size; i++) {
        result += sqrt(fabs(cos(left[i]) * sin(right[i])));
    }

    return result;
}

// Process data in blocks that fit in L1 cache
static inline double process_chunk_blocked_avx2(const int *left, const int *right, size_t size) {
    double result = 0.0;

    // Process data in blocks
    for (size_t block_start = 0; block_start < size; block_start += BLOCK_SIZE) {
        size_t block_end = (block_start + BLOCK_SIZE < size) ? block_start + BLOCK_SIZE : size;
        result += process_chunk_avx2(left + block_start, right + block_start, block_end - block_start);
    }

    return result;
}
#endif

void *compute(void *args) {
  Chunk *chunk = (Chunk *)args;
#ifdef __x86_64__
  chunk->result = process_chunk_blocked_avx2(chunk->left, chunk->right, chunk->size);
#else
  // Non-x86 implementation with blocked processing
  double result = 0.0;
  for (size_t block_start = 0; block_start < chunk->size; block_start += BLOCK_SIZE) {
    size_t block_end = (block_start + BLOCK_SIZE < chunk->size) ? block_start + BLOCK_SIZE : chunk->size;
    for (size_t i = block_start; i < block_end; i++) {
      result += sqrt(fabs(cos(chunk->left[i]) * sin(chunk->right[i])));
    }
  }
  chunk->result = result;
#endif
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

  // Align memory to cache line size
  size_t aligned_size = ALIGN_UP(size * sizeof(int), CACHE_LINE_SIZE);
  int *data = mmap(NULL, aligned_size, PROT_READ, MAP_PRIVATE, fd, 0);
  if (data == MAP_FAILED) {
    fprintf(stderr, "Error: Memory mapping failed\n");
    close(fd);
    return 1;
  }

  unsigned chunk_size = half_size / num_threads;
  unsigned chunk_size_overflow = half_size % num_threads;

  // Align chunks to cache line size
  Chunk *chunks = aligned_alloc(CACHE_LINE_SIZE, num_threads * sizeof(Chunk));
  pthread_t *threads = malloc(num_threads * sizeof(pthread_t));

  // Prefetch data into cache
#ifdef __x86_64__
  for (size_t i = 0; i < size; i += CACHE_LINE_SIZE) {
    _mm_prefetch((const char *)&data[i], _MM_HINT_T0);
  }
#endif

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

  free(threads);
  free(chunks);
  munmap(data, aligned_size);
  close(fd);

  return 0;
}
