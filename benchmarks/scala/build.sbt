name := "langbench"
version := "0.1.0"
scalaVersion := "3.3.3"

libraryDependencies += "org.scala-lang.modules" %% "scala-parallel-collections" % "1.0.4"

assembly / assemblyMergeStrategy := {
  case PathList("META-INF", xs @ _*) => MergeStrategy.discard
  case x => MergeStrategy.first
}

assembly / assemblyJarName := "langbench.jar"
