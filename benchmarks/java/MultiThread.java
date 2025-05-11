import java.io.File;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.MappedByteBuffer;
import java.nio.channels.FileChannel;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.Callable;
import java.util.List;
import java.util.ArrayList;

public class MultiThread {
    static class Chunk implements Callable<Double> {
        private final MappedByteBuffer buffer;
        private final int startIndex;
        private final int size;
        private final int halfSize;

        public Chunk(MappedByteBuffer buffer, int startIndex, int size, int halfSize) {
            this.buffer = buffer;
            this.startIndex = startIndex;
            this.size = size;
            this.halfSize = halfSize;
        }

        @Override
        public Double call() {
            double result = 0;
            for (int i = 0; i < size; i++) {
                int firstValue = buffer.getInt((startIndex + i) * Integer.BYTES);
                int secondValue = buffer.getInt((halfSize + startIndex + i) * Integer.BYTES);
                result += Math.sqrt(Math.abs(Math.cos(firstValue) * Math.sin(secondValue)));
            }
            return result;
        }
    }

    public static void main(String[] args) {
        if (args.length < 3) {
            System.err.println("Usage: java MultiThread <filepath> <size> <threads>");
            System.exit(1);
        }

        try {
            int size = Integer.parseInt(args[1]);
            if (size <= 0) {
                System.err.println("Error: Size must be a positive integer");
                System.exit(1);
            }
            int halfSize = size / 2;

            int numThreads = Integer.parseInt(args[2]);
            if (numThreads <= 0) {
                System.err.println("Error: Threads must be a positive integer");
                System.exit(1);
            }

            File file = new File(args[0]);
            RandomAccessFile raf = new RandomAccessFile(file, "r");
            FileChannel channel = raf.getChannel();

            // Map the file into memory
            MappedByteBuffer buffer = channel.map(
                FileChannel.MapMode.READ_ONLY,
                0,
                size * Integer.BYTES
            );
            buffer.order(java.nio.ByteOrder.LITTLE_ENDIAN);

            // Calculate chunk sizes
            int chunkSize = halfSize / numThreads;
            int chunkSizeOverflow = halfSize % numThreads;

            // Create thread pool
            ExecutorService executor = Executors.newFixedThreadPool(numThreads);
            Future<Double>[] futures = new Future[numThreads];

            // Submit tasks
            int currentPos = 0;
            for (int i = 0; i < numThreads; i++) {
                int actualChunkSize = chunkSize + (i < chunkSizeOverflow ? 1 : 0);
                futures[i] = executor.submit(new Chunk(buffer, currentPos, actualChunkSize, halfSize));
                currentPos += actualChunkSize;
            }

            // Collect results
            double result = 0;
            for (Future<Double> future : futures) {
                result += future.get();
            }

            System.out.printf("%f\n", result);

            // Clean up
            executor.shutdown();
            channel.close();
            raf.close();

        } catch (IOException e) {
            System.err.println("Error: " + e.getMessage());
            System.exit(1);
        } catch (NumberFormatException e) {
            System.err.println("Error: Size and threads must be valid integers");
            System.exit(1);
        } catch (Exception e) {
            System.err.println("Error: " + e.getMessage());
            System.exit(1);
        }
    }
}
