#include <iostream>
#include <fstream>
#include <vector>
#include <cstring>
#include <thread>
#include <algorithm>
#include <cmath>

void merge(std::vector<int32_t>& arr, int left, int mid, int right) {
    int n1 = mid - left + 1;
    int n2 = right - mid;

    // Create temporary arrays
    std::vector<int32_t> L(n1);
    std::vector<int32_t> R(n2);

    // Copy data to temporary arrays
    std::memcpy(L.data(), &arr[left], n1 * sizeof(int32_t));
    std::memcpy(R.data(), &arr[mid + 1], n2 * sizeof(int32_t));

    // Merge the temporary arrays back
    int i = 0, j = 0, k = left;
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
}

void mergeSortSequential(std::vector<int32_t>& arr, int left, int right) {
    if (left < right) {
        int mid = left + (right - left) / 2;
        mergeSortSequential(arr, left, mid);
        mergeSortSequential(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

void parallelMergeSort(std::vector<int32_t>& arr, int left, int right, int depth, int max_depth) {
    if (left < right) {
        int mid = left + (right - left) / 2;

        if (depth < max_depth) {
            // Create threads for left and right halves
            std::thread left_thread(parallelMergeSort, std::ref(arr), left, mid, depth + 1, max_depth);
            std::thread right_thread(parallelMergeSort, std::ref(arr), mid + 1, right, depth + 1, max_depth);

            // Wait for both threads to complete
            left_thread.join();
            right_thread.join();
        } else {
            // Sequential sorting for remaining depth
            mergeSortSequential(arr, left, mid);
            mergeSortSequential(arr, mid + 1, right);
        }

        merge(arr, left, mid, right);
    }
}

int main(int argc, char* argv[]) {
    if (argc != 5) {
        std::cerr << "Usage: " << argv[0] << " <input_file> <num_integers> <num_cores> <output_file>" << std::endl;
        return 1;
    }

    const char* input_file = argv[1];
    int num_integers = std::atoi(argv[2]);
    int num_cores = std::atoi(argv[3]);
    const char* output_file = argv[4];

    // Calculate max depth based on number of cores
    int max_depth = 0;
    int temp = num_cores;
    while (temp > 1) {
        max_depth++;
        temp /= 2;
    }

    // Allocate memory for the array
    std::vector<int32_t> arr(num_integers);
    if (arr.empty()) {
        std::cerr << "Memory allocation failed" << std::endl;
        return 1;
    }

    // Read input file
    std::ifstream in(input_file, std::ios::binary);
    if (!in) {
        std::cerr << "Error opening input file" << std::endl;
        return 1;
    }

    in.read(reinterpret_cast<char*>(arr.data()), num_integers * sizeof(int32_t));
    if (in.gcount() != num_integers * sizeof(int32_t)) {
        std::cerr << "Error reading input file" << std::endl;
        in.close();
        return 1;
    }
    in.close();

    // Perform parallel merge sort
    parallelMergeSort(arr, 0, num_integers - 1, 0, max_depth);

    // Write output file
    std::ofstream out(output_file, std::ios::binary);
    if (!out) {
        std::cerr << "Error opening output file" << std::endl;
        return 1;
    }

    out.write(reinterpret_cast<const char*>(arr.data()), num_integers * sizeof(int32_t));
    if (!out) {
        std::cerr << "Error writing output file" << std::endl;
        out.close();
        return 1;
    }
    out.close();

    return 0;
}
