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
        Array.Copy(arr, left, L, 0, n1);
        Array.Copy(arr, mid + 1, R, 0, n2);

        // Merge the temporary arrays back
        int i = 0, j = 0, k = left;
        while (i < n1 && j < n2)
        {
            if (L[i] <= R[j])
            {
                arr[k] = L[i];
                i++;
            }
            else
            {
                arr[k] = R[j];
                j++;
            }
            k++;
        }

        // Copy remaining elements of L[]
        while (i < n1)
        {
            arr[k] = L[i];
            i++;
            k++;
        }

        // Copy remaining elements of R[]
        while (j < n2)
        {
            arr[k] = R[j];
            j++;
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
            int mid = left + (right - left) / 2;  // Changed to match C version's calculation

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

    static int Main(string[] args)
    {
        if (args.Length != 4)
        {
            Console.Error.WriteLine("Usage: dotnet run <input_file> <num_integers> <num_cores> <output_file>");
            return 1;
        }

        string inputFile = args[0];
        if (!int.TryParse(args[1], out int numIntegers))
        {
            Console.Error.WriteLine("Invalid number of integers");
            return 1;
        }
        if (!int.TryParse(args[2], out int numCores))
        {
            Console.Error.WriteLine("Invalid number of cores");
            return 1;
        }
        string outputFile = args[3];

        // Calculate max depth based on number of cores (matching C version's approach)
        int maxDepth = 0;
        int temp = numCores;
        while (temp > 1)
        {
            maxDepth++;
            temp /= 2;
        }

        // Allocate array
        int[] arr = new int[numIntegers];

        try
        {
            // Read input file
            using (FileStream fs = new FileStream(inputFile, FileMode.Open, FileAccess.Read))
            {
                byte[] bytes = new byte[numIntegers * sizeof(int)];
                int bytesRead = fs.Read(bytes, 0, bytes.Length);
                if (bytesRead != bytes.Length)
                {
                    Console.Error.WriteLine("Error reading input file");
                    return 1;
                }
                Buffer.BlockCopy(bytes, 0, arr, 0, bytes.Length);
            }

            // Perform parallel merge sort
            ParallelMergeSort(arr, 0, numIntegers - 1, 0, maxDepth);

            // Write output file
            using (FileStream fs = new FileStream(outputFile, FileMode.Create, FileAccess.Write))
            {
                byte[] outputBytes = new byte[numIntegers * sizeof(int)];
                Buffer.BlockCopy(arr, 0, outputBytes, 0, outputBytes.Length);
                fs.Write(outputBytes, 0, outputBytes.Length);
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }

        return 0;
    }
}
