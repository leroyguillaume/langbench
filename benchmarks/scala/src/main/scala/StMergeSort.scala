import java.io.{FileInputStream, FileOutputStream, DataInputStream, DataOutputStream}
import java.nio.{ByteBuffer, ByteOrder}

object StMergeSort {
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

  def mergeSort(arr: Array[Int], left: Int, right: Int): Unit = {
    if (left < right) {
      val mid = left + (right - left) / 2
      mergeSort(arr, left, mid)
      mergeSort(arr, mid + 1, right)
      merge(arr, left, mid, right)
    }
  }

  def main(args: Array[String]): Unit = {
    if (args.length != 3) {
      System.err.println("Usage: scala StMergeSort <input_file> <num_integers> <output_file>")
      sys.exit(1)
    }

    val inputFile = args(0)
    val numIntegers = args(1).toInt
    val outputFile = args(2)

    // Allocate array for integers
    val arr = new Array[Int](numIntegers)

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

      // Perform merge sort
      mergeSort(arr, 0, numIntegers - 1)

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
    }
  }
}
