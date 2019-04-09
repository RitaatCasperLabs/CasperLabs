package io.casperlabs.node.diagnostics.client

import java.io.Closeable
import java.util.concurrent.TimeUnit

import com.google.protobuf.ByteString
import com.google.protobuf.empty.Empty
import io.casperlabs.comm.discovery.Node
import io.casperlabs.node.api.diagnostics._
import io.grpc.{ManagedChannel, ManagedChannelBuilder}
import monix.eval.Task

trait DiagnosticsService[F[_]] {
  def listPeers: F[Seq[Node]]
  def listDiscoveredPeers: F[Seq[Node]]
  def nodeCoreMetrics: F[NodeCoreMetrics]
  def processCpu: F[ProcessCpu]
  def memoryUsage: F[MemoryUsage]
  def garbageCollectors: F[Seq[GarbageCollector]]
  def memoryPools: F[Seq[MemoryPool]]
  def threads: F[Threads]
}

object DiagnosticsService {
  def apply[F[_]](implicit ev: DiagnosticsService[F]): DiagnosticsService[F] = ev
}

class GrpcDiagnosticsService(host: String, port: Int, maxMessageSize: Int)
    extends DiagnosticsService[Task]
    with Closeable {

  private val channel: ManagedChannel =
    ManagedChannelBuilder
      .forAddress(host, port)
      .maxInboundMessageSize(maxMessageSize)
      .usePlaintext()
      .build

  private val stub = DiagnosticsGrpcMonix.stub(channel)

  def listPeers: Task[Seq[Node]] =
    stub
      .listPeers(Empty())
      .map(
        _.peers.map(
          p =>
            Node(
              ByteString.copyFrom(p.key.toByteArray),
              p.host,
              p.port,
              p.port
            )
        )
      )

  def listDiscoveredPeers: Task[Seq[Node]] =
    stub
      .listDiscoveredPeers(Empty())
      .map(
        _.peers
          .map(
            p =>
              Node(
                ByteString.copyFrom(p.key.toByteArray),
                p.host,
                p.port,
                p.port
              )
          )
      )

  def nodeCoreMetrics: Task[NodeCoreMetrics] =
    stub.getNodeCoreMetrics(Empty())

  def processCpu: Task[ProcessCpu] =
    stub.getProcessCpu(Empty())

  def memoryUsage: Task[MemoryUsage] =
    stub.getMemoryUsage(Empty())

  def garbageCollectors: Task[Seq[GarbageCollector]] =
    stub.getGarbageCollectors(Empty()) map (_.garbageCollectors)

  def memoryPools: Task[Seq[MemoryPool]] =
    stub.getMemoryPools(Empty()).map(_.memoryPools)

  def threads: Task[Threads] =
    stub.getThreads(Empty())

  override def close(): Unit = {
    val terminated = channel.shutdown().awaitTermination(10, TimeUnit.SECONDS)
    if (!terminated) {
      println(
        "warn: did not shutdown after 10 seconds, retrying with additional 10 seconds timeout"
      )
      channel.awaitTermination(10, TimeUnit.SECONDS)
    }
  }

}
