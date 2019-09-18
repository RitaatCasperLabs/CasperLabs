package io.casperlabs.comm.rp

import cats.Id
import com.google.protobuf.ByteString
import io.casperlabs.catscontrib.effect.implicits._
import io.casperlabs.catscontrib.ski._
import io.casperlabs.comm.CommError._
import io.casperlabs.comm.discovery.Node
import io.casperlabs.comm.protocol.routing._
import io.casperlabs.comm.rp.Connect._
import io.casperlabs.comm.rp.ProtocolHelper._
import io.casperlabs.metrics.Metrics
import io.casperlabs.p2p.EffectsTestInstances.{LogicalTime, TransportLayerStub}
import io.casperlabs.shared._
import org.scalatest._

import scala.concurrent.duration._

class ResestConnectionsSpec
    extends FunSpec
    with Matchers
    with BeforeAndAfterEach
    with AppendedClues {

  val src: Node          = peer("src")
  implicit val transport = new TransportLayerStub[Id]
  implicit val log       = new Log.NOPLog[Id]
  implicit val metric    = new Metrics.MetricsNOP[Id]
  implicit val time      = new LogicalTime[Id]

  override def beforeEach(): Unit = {
    transport.reset()
    transport.setResponses(kp(alwaysSuccess))
  }

  describe("if reset reconnections") {
    it("should disconnect from all peers and clear connections") {
      // given
      implicit val connections = mkConnections(peer("A"), peer("B"))
      implicit val rpconf =
        conf(maxNumOfConnections = 5)
      // when
      Connect.resetConnections[Id]
      // then
      connections.read.size shouldBe 0
      transport.requests.size shouldBe 2
      transport.requests.map(_.peer) should contain(peer("A"))
      transport.requests.map(_.peer) should contain(peer("B"))
      transport.requests.forall(_.msg.message.isDisconnect) shouldEqual true
      transport.disconnects.size shouldBe 2
      transport.disconnects should contain(peer("A"))
      transport.disconnects should contain(peer("B"))
    }
  }

  private def peer(name: String, host: String = "host"): Node =
    Node(ByteString.copyFrom(name.getBytes), host, 80, 80)

  private def mkConnections(peers: Node*): ConnectionsCell[Id] =
    Cell.id[Connections](peers.toList)

  private def conf(
      maxNumOfConnections: Int,
      numOfConnectionsPinged: Int = 5
  ): RPConfAsk[Id] =
    new ConstApplicativeAsk(
      RPConf(
        clearConnections = ClearConnectionsConf(maxNumOfConnections, numOfConnectionsPinged),
        defaultTimeout = 1.milli,
        local = peer("src"),
        bootstraps = Nil
      )
    )

  def alwaysSuccess: Protocol => CommErr[Protocol] =
    kp(Right(heartbeat(peer("src"))))

}
