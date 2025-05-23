#include <iostream>
#include <fstream>
#include <vector>
#include <cstring>

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

void mergeSort(std::vector<int32_t>& arr, int left, int right) {
    if (left < right) {
        int mid = left + (right - left) / 2;
        mergeSort(arr, left, mid);
        mergeSort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
}

int main(int argc, char* argv[]) {
    if (argc != 4) {
        std::cerr << "Usage: " << argv[0] << " <input_file> <num_integers> <output_file>" << std::endl;
        return 1;
    }

    const char* input_file = argv[1];
    int num_integers = std::atoi(argv[2]);
    const char* output_file = argv[3];

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

    // Perform merge sort
    mergeSort(arr, 0, num_integers - 1);

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
