import java.io.{FileInputStream, FileOutputStream}
import java.nio.{ByteBuffer, ByteOrder}
import java.util.concurrent.{Executors, ExecutorService, Future => JFuture}
import scala.concurrent.{Future, Await}
import scala.concurrent.duration._
import scala.concurrent.ExecutionContext.Implicits.global

object MtMergeSort {
  def merge(arr: Array[Int], left: Int, mid: Int, right: Int): Unit = {
    val n1 = mid - left + 1
    val n2 = right - mid

    // Create temporary arrays
    val L = new Array[Int](n1)
    val R = new Array[Int](n2)

    // Copy data to temporary arrays
    System.arraycopy(arr, left, L, 0, n1)
    System.arraycopy(arr, mid + 1, R, 0, n2)

    // Merge the temporary arrays back
    var i = 0
    var j = 0
    var k = left
    while (i < n1 && j < n2) {
      if (L(i) <= R(j)) {
        arr(k) = L(i)
        i += 1
      } else {
        arr(k) = R(j)
        j += 1
      }
      k += 1
    }

    // Copy remaining elements of L[]
    while (i < n1) {
      arr(k) = L(i)
      i += 1
      k += 1
    }

    // Copy remaining elements of R[]
    while (j < n2) {
      arr(k) = R(j)
      j += 1
      k += 1
    }
  }

  def mergeSortThread(arr: Array[Int], left: Int, right: Int, depth: Int, maxDepth: Int, executor: ExecutorService): Unit = {
    if (left < right) {
      val mid = left + (right - left) / 2

      if (depth < maxDepth) {
        // Create futures for left and right halves
        val leftFuture = executor.submit(new Runnable {
          def run(): Unit = mergeSortThread(arr, left, mid, depth + 1, maxDepth, executor)
        })
        val rightFuture = executor.submit(new Runnable {
          def run(): Unit = mergeSortThread(arr, mid + 1, right, depth + 1, maxDepth, executor)
        })

        // Wait for both futures to complete
        leftFuture.get()
        rightFuture.get()
      } else {
        // Sequential sorting for remaining depth
        mergeSortThread(arr, left, mid, depth + 1, maxDepth, executor)
        mergeSortThread(arr, mid + 1, right, depth + 1, maxDepth, executor)
      }

      merge(arr, left, mid, right)
    }
  }

  def main(args: Array[String]): Unit = {
    if (args.length != 4) {
      System.err.println("Usage: scala MtMergeSort <input_file> <num_integers> <num_cores> <output_file>")
      sys.exit(1)
    }

    val inputFile = args(0)
    val numIntegers = args(1).toInt
    val numCores = args(2).toInt
    val outputFile = args(3)

    // Calculate max depth for thread creation
    var maxDepth = 0
    var temp = numCores
    while (temp > 1) {
      maxDepth += 1
      temp /= 2
    }

    // Allocate array for integers
    val arr = new Array[Int](numIntegers)
    val executor = Executors.newFixedThreadPool(numCores)

    try {
      // Read input file
      val fis = new FileInputStream(inputFile)
      val bytes = new Array[Byte](numIntegers * 4)
      val bytesRead = fis.read(bytes)
      fis.close()

      if (bytesRead != numIntegers * 4) {
        System.err.println("Error reading input file")
        sys.exit(1)
      }

      // Convert bytes to integers
      val bb = ByteBuffer.wrap(bytes)
      bb.order(ByteOrder.LITTLE_ENDIAN)
      bb.asIntBuffer().get(arr)

      // Perform parallel merge sort
      mergeSortThread(arr, 0, numIntegers - 1, 0, maxDepth, executor)

      // Write output file
      val fos = new FileOutputStream(outputFile)
      val outBuffer = ByteBuffer.allocate(numIntegers * 4)
      outBuffer.order(ByteOrder.LITTLE_ENDIAN)
      outBuffer.asIntBuffer().put(arr)
      fos.write(outBuffer.array())
      fos.close()
    } catch {
      case e: java.io.FileNotFoundException =>
        System.err.println("Error opening file: " + e.getMessage)
        sys.exit(1)
      case e: Exception =>
        System.err.println("Error: " + e.getMessage)
        sys.exit(1)
    } finally {
      executor.shutdown()
      executor.awaitTermination(1, java.util.concurrent.TimeUnit.MINUTES)
    }
  }
}
