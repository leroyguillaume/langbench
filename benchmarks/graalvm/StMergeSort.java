import java.io.*;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;

public class StMergeSort {
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
        if (args.length != 3) {
            System.err.println("Usage: java StMergeSort <input_file> <num_integers> <output_file>");
            System.exit(1);
        }

        String inputFile = args[0];
        int numIntegers = Integer.parseInt(args[1]);
        String outputFile = args[2];

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

            // Perform merge sort
            mergeSort(arr, 0, numIntegers - 1);

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
