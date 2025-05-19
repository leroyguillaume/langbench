using System;
using System.IO;
using System.Threading.Tasks;

class MtMergeSort
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

    static void ParallelMergeSort(int[] arr, int left, int right, int depth, int maxDepth)
    {
        if (left < right)
        {
            int mid = (left + right) / 2;

            if (depth < maxDepth)
            {
                // Create tasks for parallel execution
                var leftTask = Task.Run(() => ParallelMergeSort(arr, left, mid, depth + 1, maxDepth));
                var rightTask = Task.Run(() => ParallelMergeSort(arr, mid + 1, right, depth + 1, maxDepth));

                // Wait for both tasks to complete
                Task.WaitAll(leftTask, rightTask);
            }
            else
            {
                // Sequential execution for remaining depth
                ParallelMergeSort(arr, left, mid, depth + 1, maxDepth);
                ParallelMergeSort(arr, mid + 1, right, depth + 1, maxDepth);
            }

            Merge(arr, left, mid, right);
        }
    }

    static void Main(string[] args)
    {
        if (args.Length != 4)
        {
            Console.WriteLine("Usage: dotnet run <input_file> <num_integers> <num_cores> <output_file>");
            Environment.Exit(1);
        }

        string inputFile = args[0];
        int numIntegers = int.Parse(args[1]);
        int numCores = int.Parse(args[2]);
        string outputFile = args[3];

        // Calculate max depth based on number of cores
        int maxDepth = (int)Math.Log2(numCores);

        // Read input file
        byte[] bytes = File.ReadAllBytes(inputFile);
        int[] arr = new int[numIntegers];
        Buffer.BlockCopy(bytes, 0, arr, 0, numIntegers * sizeof(int));

        // Perform parallel merge sort
        ParallelMergeSort(arr, 0, numIntegers - 1, 0, maxDepth);

        // Write output file
        byte[] outputBytes = new byte[numIntegers * sizeof(int)];
        Buffer.BlockCopy(arr, 0, outputBytes, 0, numIntegers * sizeof(int));
        File.WriteAllBytes(outputFile, outputBytes);
    }
}
