import java.io.File;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.MappedByteBuffer;
import java.nio.channels.FileChannel;

public class SingleThread {
    public static void main(String[] args) {
        if (args.length < 2) {
            System.err.println("Usage: java SingleThread <filepath> <size>");
            System.exit(1);
        }

        try {
            int size = Integer.parseInt(args[1]);
            if (size <= 0) {
                System.err.println("Error: Size must be a positive integer");
                System.exit(1);
            }
            int halfSize = size / 2;

            File file = new File(args[0]);
            RandomAccessFile raf = new RandomAccessFile(file, "r");
            FileChannel channel = raf.getChannel();

            // Map the file into memory
            MappedByteBuffer buffer = channel.map(
                FileChannel.MapMode.READ_ONLY,
                0,
                size * Integer.BYTES
            );

            double result = 0;
            for (int i = 0; i < halfSize; i++) {
                buffer.order(java.nio.ByteOrder.LITTLE_ENDIAN);
                int firstValue = buffer.getInt(i * Integer.BYTES);
                int secondValue = buffer.getInt((halfSize + i) * Integer.BYTES);
                result += Math.sqrt(Math.abs(Math.cos(firstValue) * Math.sin(secondValue)));
            }

            System.out.printf("%f\n", result);

            // Clean up
            channel.close();
            raf.close();

        } catch (IOException e) {
            System.err.println("Error: " + e.getMessage());
            System.exit(1);
        } catch (NumberFormatException e) {
            System.err.println("Error: Size must be a valid integer");
            System.exit(1);
        }
    }
}
