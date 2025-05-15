#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>

// Structure to pass arguments to the thread function
typedef struct {
    int* arr;
    int left;
    int right;
    int depth;
    int max_depth;
} ThreadArgs;

void merge(int arr[], int left, int mid, int right) {
    int i, j, k;
    int n1 = mid - left + 1;
    int n2 = right - mid;

    // Create temporary arrays
    int* L = (int*)malloc(n1 * sizeof(int));
    int* R = (int*)malloc(n2 * sizeof(int));

    // Copy data to temporary arrays
    memcpy(L, &arr[left], n1 * sizeof(int));
    memcpy(R, &arr[mid + 1], n2 * sizeof(int));

    // Merge the temporary arrays back
    i = 0;
    j = 0;
    k = left;
    while (i < n1 && j < n2) {
        if (L[i] <= R[j]) {
            arr[k] = L[i];
            i++;
        } else {
            arr[k] = R[j];
            j++;
        }
        k++;
    }

    // Copy remaining elements of L[]
    while (i < n1) {
        arr[k] = L[i];
        i++;
        k++;
    }

    // Copy remaining elements of R[]
    while (j < n2) {
        arr[k] = R[j];
        j++;
        k++;
    }

    free(L);
    free(R);
}

void* mergeSortThread(void* arg) {
    ThreadArgs* args = (ThreadArgs*)arg;
    int* arr = args->arr;
    int left = args->left;
    int right = args->right;
    int depth = args->depth;
    int max_depth = args->max_depth;

    if (left < right) {
        int mid = left + (right - left) / 2;

        if (depth < max_depth) {
            // Create threads for left and right halves
            pthread_t left_thread, right_thread;
            ThreadArgs left_args = {arr, left, mid, depth + 1, max_depth};
            ThreadArgs right_args = {arr, mid + 1, right, depth + 1, max_depth};

            pthread_create(&left_thread, NULL, mergeSortThread, &left_args);
            pthread_create(&right_thread, NULL, mergeSortThread, &right_args);

            pthread_join(left_thread, NULL);
            pthread_join(right_thread, NULL);
        } else {
            // Sequential sorting for remaining depth
            ThreadArgs left_args = {arr, left, mid, depth + 1, max_depth};
            ThreadArgs right_args = {arr, mid + 1, right, depth + 1, max_depth};
            mergeSortThread(&left_args);
            mergeSortThread(&right_args);
        }

        merge(arr, left, mid, right);
    }

    return NULL;
}

int main(int argc, char* argv[]) {
    if (argc != 5) {
        fprintf(stderr, "Usage: %s <input_file> <num_integers> <num_cores> <output_file>\n", argv[0]);
        return 1;
    }

    const char* input_file = argv[1];
    int num_integers = atoi(argv[2]);
    int num_cores = atoi(argv[3]);
    const char* output_file = argv[4];

    // Calculate max depth for thread creation
    int max_depth = 0;
    int temp = num_cores;
    while (temp > 1) {
        max_depth++;
        temp /= 2;
    }

    // Allocate memory for the array
    int* arr = (int*)malloc(num_integers * sizeof(int));
    if (arr == NULL) {
        fprintf(stderr, "Memory allocation failed\n");
        return 1;
    }

    // Read input file
    FILE* fp_in = fopen(input_file, "rb");
    if (fp_in == NULL) {
        fprintf(stderr, "Error opening input file\n");
        free(arr);
        return 1;
    }

    size_t read_size = fread(arr, sizeof(int), num_integers, fp_in);
    if (read_size != num_integers) {
        fprintf(stderr, "Error reading input file\n");
        fclose(fp_in);
        free(arr);
        return 1;
    }
    fclose(fp_in);

    // Perform parallel merge sort
    ThreadArgs args = {arr, 0, num_integers - 1, 0, max_depth};
    mergeSortThread(&args);

    // Write output file
    FILE* fp_out = fopen(output_file, "wb");
    if (fp_out == NULL) {
        fprintf(stderr, "Error opening output file\n");
        free(arr);
        return 1;
    }

    size_t write_size = fwrite(arr, sizeof(int), num_integers, fp_out);
    if (write_size != num_integers) {
        fprintf(stderr, "Error writing output file\n");
        fclose(fp_out);
        free(arr);
        return 1;
    }
    fclose(fp_out);

    // Clean up
    free(arr);
    return 0;
}
