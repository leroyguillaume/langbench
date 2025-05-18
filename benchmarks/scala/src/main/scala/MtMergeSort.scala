import java.io.{FileInputStream, FileOutputStream}
import java.nio.{ByteBuffer, ByteOrder}
import scala.concurrent.{Future, Await}
import scala.concurrent.duration._
import scala.concurrent.ExecutionContext.Implicits.global
import java.util.concurrent.{Executors, ExecutorService}
import scala.concurrent.ExecutionContext

object MtMergeSort {
  def merge(arr: Array[Int], left: Int, mid: Int, right: Int): Unit = {
    val n1 = mid - left + 1
    val n2 = right - mid

    // Create temporary arrays
    val L = new Array[Int](n1)
    val R = new Array[Int](n2)

    // Copy data to temporary arrays using System.arraycopy
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

  def mergeSortSequential(arr: Array[Int], left: Int, right: Int): Unit = {
    if (left < right) {
      val mid = left + (right - left) / 2  // Avoid integer overflow
      mergeSortSequential(arr, left, mid)
      mergeSortSequential(arr, mid + 1, right)
      merge(arr, left, mid, right)
    }
  }

  def mergeSortedArrays(arr1: Array[Int], arr2: Array[Int]): Array[Int] = {
    val result = new Array[Int](arr1.length + arr2.length)
    var i = 0
    var j = 0
    var k = 0

    while (i < arr1.length && j < arr2.length) {
      if (arr1(i) <= arr2(j)) {
        result(k) = arr1(i)
        i += 1
      } else {
        result(k) = arr2(j)
        j += 1
      }
      k += 1
    }

    // Copy remaining elements using System.arraycopy
    if (i < arr1.length) {
      System.arraycopy(arr1, i, result, k, arr1.length - i)
    }
    if (j < arr2.length) {
      System.arraycopy(arr2, j, result, k, arr2.length - j)
    }

    result
  }

  def mergeSortParallel(arr: Array[Int], numWorkers: Int): Array[Int] = {
    val executor = Executors.newFixedThreadPool(numWorkers)
    implicit val ec = ExecutionContext.fromExecutor(executor)

    try {
      // Calculate chunk sizes ensuring even distribution
      val baseChunkSize = arr.length / numWorkers
      val remainder = arr.length % numWorkers
      val chunks = new Array[Array[Int]](numWorkers)

      var start = 0
      for (i <- 0 until numWorkers) {
        val chunkSize = baseChunkSize + (if (i < remainder) 1 else 0)
        chunks(i) = new Array[Int](chunkSize)
        System.arraycopy(arr, start, chunks(i), 0, chunkSize)
        start += chunkSize
      }

      // Sort chunks in parallel using Future with our executor
      val sortedChunks = chunks.map { chunk =>
        Future {
          mergeSortSequential(chunk, 0, chunk.length - 1)
          chunk
        }
      }

      // Wait for all chunks to be sorted
      val sortedArrays = Await.result(Future.sequence(sortedChunks), Duration.Inf).toArray

      // Merge sorted chunks in parallel using a divide-and-conquer approach
      def mergePairs(arrays: Array[Array[Int]]): Array[Array[Int]] = {
        if (arrays.length <= 1) arrays
        else {
          val pairs = arrays.grouped(2).toArray
          val mergedPairs = pairs.map { pair =>
            if (pair.length == 2) {
              Future {
                mergeSortedArrays(pair(0), pair(1))
              }
            } else {
              Future.successful(pair(0))
            }
          }
          Await.result(Future.sequence(mergedPairs), Duration.Inf).toArray
        }
      }

      // Recursively merge pairs until we have one array
      var currentArrays = sortedArrays
      while (currentArrays.length > 1) {
        currentArrays = mergePairs(currentArrays)
      }

      currentArrays(0)
    } finally {
      executor.shutdown()
      executor.awaitTermination(1, java.util.concurrent.TimeUnit.MINUTES)
    }
  }

  def main(args: Array[String]): Unit = {
    if (args.length != 4) {
      println("Usage: scala MtMergeSort <input_file> <num_integers> <num_cores> <output_file>")
      sys.exit(1)
    }

    val inputFile = args(0)
    val numIntegers = args(1).toInt
    val numCores = args(2).toInt
    val outputFile = args(3)

    try {
      // Read input file using ByteBuffer with LITTLE_ENDIAN ordering
      val fis = new FileInputStream(inputFile)
      val bytes = new Array[Byte](numIntegers * 4)
      fis.read(bytes)
      fis.close()

      val bb = ByteBuffer.wrap(bytes)
      bb.order(ByteOrder.LITTLE_ENDIAN)
      val arr = new Array[Int](numIntegers)
      bb.asIntBuffer().get(arr)

      // Perform parallel merge sort
      val sortedArr = mergeSortParallel(arr, numCores)

      // Write output file using ByteBuffer with LITTLE_ENDIAN ordering
      val fos = new FileOutputStream(outputFile)
      val outBuffer = ByteBuffer.allocate(numIntegers * 4)
      outBuffer.order(ByteOrder.LITTLE_ENDIAN)
      outBuffer.asIntBuffer().put(sortedArr)
      fos.write(outBuffer.array())
      fos.close()
    } catch {
      case e: Exception =>
        println(s"Error: ${e.getMessage}")
        sys.exit(1)
    }
  }
}
