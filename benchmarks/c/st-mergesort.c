#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

void mergeSort(int arr[], int left, int right) {
    if (left < right) {
        int mid = left + (right - left) / 2;
        mergeSort(arr, left, mid);
        mergeSort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

int main(int argc, char* argv[]) {
    if (argc != 4) {
        fprintf(stderr, "Usage: %s <input_file> <num_integers> <output_file>\n", argv[0]);
        return 1;
    }

    const char* input_file = argv[1];
    int num_integers = atoi(argv[2]);
    const char* output_file = argv[3];

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

    // Perform merge sort
    mergeSort(arr, 0, num_integers - 1);

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
