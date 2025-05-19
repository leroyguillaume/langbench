import java.io.*;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.RecursiveAction;

public class MtMergeSort {
    static class MergeSortTask extends RecursiveAction {
        private final int[] arr;
        private final int left;
        private final int right;
        private final int depth;
        private final int maxDepth;

        MergeSortTask(int[] arr, int left, int right, int depth, int maxDepth) {
            this.arr = arr;
            this.left = left;
            this.right = right;
            this.depth = depth;
            this.maxDepth = maxDepth;
        }

        @Override
        protected void compute() {
            if (left < right) {
                int mid = left + (right - left) / 2;

                if (depth < maxDepth) {
                    // Create tasks for left and right halves
                    MergeSortTask leftTask = new MergeSortTask(arr, left, mid, depth + 1, maxDepth);
                    MergeSortTask rightTask = new MergeSortTask(arr, mid + 1, right, depth + 1, maxDepth);

                    // Fork both tasks
                    leftTask.fork();
                    rightTask.fork();

                    // Join both tasks
                    leftTask.join();
                    rightTask.join();
                } else {
                    // Sequential sorting for remaining depth
                    mergeSort(arr, left, mid);
                    mergeSort(arr, mid + 1, right);
                }

                merge(arr, left, mid, right);
            }
        }
    }

    public static void merge(int[] arr, int left, int mid, int right) {
        int n1 = mid - left + 1;
        int n2 = right - mid;

        // Create temporary arrays
        int[] L = new int[n1];
        int[] R = new int[n2];

        // Copy data to temporary arrays
        System.arraycopy(arr, left, L, 0, n1);
        System.arraycopy(arr, mid + 1, R, 0, n2);

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

    public static void mergeSort(int[] arr, int left, int right) {
        if (left < right) {
            int mid = left + (right - left) / 2;
            mergeSort(arr, left, mid);
            mergeSort(arr, mid + 1, right);
            merge(arr, left, mid, right);
        }
    }

    public static void main(String[] args) {
        if (args.length != 4) {
            System.err.println("Usage: java MtMergeSort <input_file> <num_integers> <num_cores> <output_file>");
            System.exit(1);
        }

        String inputFile = args[0];
        int numIntegers = Integer.parseInt(args[1]);
        int numCores = Integer.parseInt(args[2]);
        String outputFile = args[3];

        // Calculate max depth for thread creation
        int maxDepth = 0;
        int temp = numCores;
        while (temp > 1) {
            maxDepth++;
            temp /= 2;
        }

        try {
            // Read input file
            FileInputStream fis = new FileInputStream(inputFile);
            byte[] bytes = new byte[numIntegers * 4];
            fis.read(bytes);
            fis.close();

            // Convert bytes to integers
            ByteBuffer bb = ByteBuffer.wrap(bytes);
            bb.order(ByteOrder.LITTLE_ENDIAN);
            IntBuffer ib = bb.asIntBuffer();
            int[] arr = new int[numIntegers];
            ib.get(arr);

            // Create ForkJoinPool with specified number of threads
            ForkJoinPool pool = new ForkJoinPool(numCores);

            // Perform parallel merge sort
            MergeSortTask task = new MergeSortTask(arr, 0, numIntegers - 1, 0, maxDepth);
            pool.invoke(task);
            pool.shutdown();
            pool.close();

            // Write output file
            FileOutputStream fos = new FileOutputStream(outputFile);
            ByteBuffer outBuffer = ByteBuffer.allocate(numIntegers * 4);
            outBuffer.order(ByteOrder.LITTLE_ENDIAN);
            outBuffer.asIntBuffer().put(arr);
            fos.write(outBuffer.array());
            fos.close();

        } catch (IOException e) {
            System.err.println("Error: " + e.getMessage());
            System.exit(1);
        }
    }
}
