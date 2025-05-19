using System;
using System.IO;

class StMergeSort
{
    static void Merge(int[] arr, int left, int mid, int right)
    {
        int n1 = mid - left + 1;
        int n2 = right - mid;

        // Create temporary arrays
        int[] L = new int[n1];
        int[] R = new int[n2];

        // Copy data to temporary arrays
        for (int i = 0; i < n1; i++)
            L[i] = arr[left + i];
        for (int j = 0; j < n2; j++)
            R[j] = arr[mid + 1 + j];

        // Merge the temporary arrays back
        int leftIdx = 0, rightIdx = 0, k = left;
        while (leftIdx < n1 && rightIdx < n2)
        {
            if (L[leftIdx] <= R[rightIdx])
            {
                arr[k] = L[leftIdx];
                leftIdx++;
            }
            else
            {
                arr[k] = R[rightIdx];
                rightIdx++;
            }
            k++;
        }

        // Copy remaining elements of L[]
        while (leftIdx < n1)
        {
            arr[k] = L[leftIdx];
            leftIdx++;
            k++;
        }

        // Copy remaining elements of R[]
        while (rightIdx < n2)
        {
            arr[k] = R[rightIdx];
            rightIdx++;
            k++;
        }
    }

    static void MergeSort(int[] arr, int left, int right)
    {
        if (left < right)
        {
            int mid = (left + right) / 2;
            MergeSort(arr, left, mid);
            MergeSort(arr, mid + 1, right);
            Merge(arr, left, mid, right);
        }
    }

    static void Main(string[] args)
    {
        if (args.Length != 3)
        {
            Console.WriteLine("Usage: dotnet run <input_file> <num_integers> <output_file>");
            Environment.Exit(1);
        }

        string inputFile = args[0];
        int numIntegers = int.Parse(args[1]);
        string outputFile = args[2];

        // Read input file
        byte[] bytes = File.ReadAllBytes(inputFile);
        int[] arr = new int[numIntegers];
        Buffer.BlockCopy(bytes, 0, arr, 0, numIntegers * sizeof(int));

        // Perform merge sort
        MergeSort(arr, 0, numIntegers - 1);

        // Write output file
        byte[] outputBytes = new byte[numIntegers * sizeof(int)];
        Buffer.BlockCopy(arr, 0, outputBytes, 0, numIntegers * sizeof(int));
        File.WriteAllBytes(outputFile, outputBytes);
    }
}
