package io.casperlabs.casper

import cats.data.NonEmptyList
import cats.effect.concurrent.Semaphore
import cats.effect.{Concurrent, Resource, Sync}
import cats.implicits._
import cats.mtl.FunctorRaise
import cats.Applicative
import com.github.ghik.silencer.silent
import com.google.protobuf.ByteString
import io.casperlabs.casper.DeploySelection.DeploySelection
import io.casperlabs.casper.Estimator.BlockHash
import io.casperlabs.casper.consensus._
import io.casperlabs.casper.consensus.state.ProtocolVersion
import io.casperlabs.casper.consensus.Block.Justification
import io.casperlabs.casper.DeployFilters.filterDeploysNotInPast
import io.casperlabs.casper.equivocations.EquivocationDetector
import io.casperlabs.casper.finality.CommitteeWithConsensusValue
import io.casperlabs.casper.finality.votingmatrix.FinalityDetectorVotingMatrix
import io.casperlabs.casper.util.ProtoUtil._
import io.casperlabs.casper.util.ProtocolVersions.Config
import io.casperlabs.casper.util._
import io.casperlabs.casper.util.execengine.ExecEngineUtil
import io.casperlabs.casper.validation.Errors._
import io.casperlabs.casper.validation.Validation
import io.casperlabs.catscontrib._
import io.casperlabs.comm.gossiping
import io.casperlabs.crypto.Keys
import io.casperlabs.crypto.signatures.SignatureAlgorithm
import io.casperlabs.ipc
import io.casperlabs.ipc.ChainSpec.DeployConfig
import io.casperlabs.mempool.DeployBuffer
import io.casperlabs.metrics.Metrics
import io.casperlabs.metrics.implicits._
import io.casperlabs.models.{Message, SmartContractEngineError}
import io.casperlabs.shared._
import io.casperlabs.smartcontracts.ExecutionEngineService
import io.casperlabs.storage.BlockMsgWithTransform
import io.casperlabs.storage.block.BlockStorage
import io.casperlabs.storage.dag.{DagRepresentation, DagStorage}
import io.casperlabs.storage.deploy.{DeployStorage, DeployStorageReader, DeployStorageWriter}
import simulacrum.typeclass
import io.casperlabs.models.BlockImplicits._
import io.casperlabs.catscontrib.effect.implicits.fiberSyntax

import scala.concurrent.duration.FiniteDuration
import scala.util.control.NonFatal

/**
  * Encapsulates mutable state of the MultiParentCasperImpl
  *
  * @param invalidBlockTracker
  */
final case class CasperState(
    invalidBlockTracker: Set[BlockHash] = Set.empty[BlockHash],
    dependencyDag: DoublyLinkedDag[BlockHash] = BlockDependencyDag.empty
)

@silent("is never used")
class MultiParentCasperImpl[F[_]: Concurrent: Log: Metrics: Time: BlockStorage: DagStorage: DeployBuffer: ExecutionEngineService: LastFinalizedBlockHashContainer: FinalityDetectorVotingMatrix: DeployStorage: Validation: Fs2Compiler: DeploySelection: CasperLabsProtocol](
    validatorSemaphoreMap: SemaphoreMap[F, ByteString],
    statelessExecutor: MultiParentCasperImpl.StatelessExecutor[F],
    validatorId: Option[ValidatorIdentity],
    genesis: Block,
    chainName: String,
    minTtl: FiniteDuration,
    upgrades: Seq[ipc.ChainSpec.UpgradePoint]
)(implicit state: Cell[F, CasperState])
    extends MultiParentCasper[F] {

  import MultiParentCasperImpl._

  implicit val logSource     = LogSource(this.getClass)
  implicit val metricsSource = CasperMetricsSource

  //TODO pull out
  implicit val functorRaiseInvalidBlock = validation.raiseValidateErrorThroughApplicativeError[F]

  type Validator = ByteString

  /** Add a block if it hasn't been added yet. */
  def addBlock(
      block: Block
  ): F[BlockStatus] = {
    def addBlock(
        validateAndAddBlock: (
            Option[StatelessExecutor.Context],
            Block
        ) => F[BlockStatus]
    ): F[BlockStatus] =
      validatorSemaphoreMap.withPermit(block.getHeader.validatorPublicKey) {
        for {
          dag       <- DagStorage[F].getRepresentation
          blockHash = block.blockHash
          inDag     <- dag.contains(blockHash)
          status <- if (inDag) {
                     Log[F]
                       .info(
                         s"Block ${PrettyPrinter.buildString(blockHash)} has already been processed by another thread."
                       ) *>
                       BlockStatus.processed.pure[F]
                   } else {
                     // This might be the first time we see this block, or it may not have been added to the state
                     // because it was an IgnorableEquivocation, but then we saw a child and now we got it again.
                     internalAddBlock(block, dag, validateAndAddBlock)
                   }
        } yield status
      }

    val handleInvalidTimestamp =
      (_: Option[StatelessExecutor.Context], block: Block) =>
        statelessExecutor
          .addEffects(InvalidUnslashableBlock, block, Seq.empty)
          .as(InvalidUnslashableBlock: BlockStatus)

    // If the block timestamp is in the future, wait some time before adding it,
    // so we won't include it as a justification from the future.
    Validation.preTimestamp[F](block).attempt.flatMap {
      case Right(None) =>
        addBlock(statelessExecutor.validateAndAddBlock)
      case Right(Some(delay)) =>
        Log[F].info(
          s"${PrettyPrinter.buildString(block.blockHash) -> "block"} is ahead for $delay from now, will retry adding later"
        ) >>
          Time[F].sleep(delay) >>
          addBlock(statelessExecutor.validateAndAddBlock)
      case _ =>
        Log[F]
          .warn(
            s"${PrettyPrinter.buildString(block.blockHash) -> "block"} timestamp exceeded threshold"
          ) >>
          addBlock(handleInvalidTimestamp)
    }
  }

  /** Validate the block, try to execute and store it,
    * execute any other block that depended on it,
    * update the finalized block reference. */
  private def internalAddBlock(
      block: Block,
      dag: DagRepresentation[F],
      validateAndAddBlock: (
          Option[StatelessExecutor.Context],
          Block
      ) => F[BlockStatus]
  ): F[BlockStatus] = Metrics[F].timer("addBlock") {
    for {
      lastFinalizedBlockHash <- LastFinalizedBlockHashContainer[F].get
      status <- validateAndAddBlock(
                 StatelessExecutor.Context(genesis, lastFinalizedBlockHash).some,
                 block
               )
      _          <- removeAdded(List(block -> status), canRemove = _ != MissingBlocks)
      hashPrefix = PrettyPrinter.buildString(block.blockHash)
      // Update the last finalized block; remove finalized deploys from the buffer
      _ <- Log[F].debug(s"Updating last finalized block after adding ${hashPrefix -> "block"}")
      updatedLFB <- if (status == Valid) updateLastFinalizedBlock(block, dag)
                   else false.pure[F]
      // Remove any deploys from the buffer which are in finalized blocks.
      _ <- {
        Log[F]
          .debug(s"Removing finalized deploys after adding ${hashPrefix -> "block"}") *>
          LastFinalizedBlockHashContainer[F].get >>= { lfb =>
          DeployBuffer.removeFinalizedDeploys[F](lfb).forkAndLog
        }
      }.whenA(updatedLFB)
      _ <- Log[F].debug(s"Finished adding ${hashPrefix -> "block"}")
    } yield status
  }

  /** Update the finalized block; return true if it changed. */
  private def updateLastFinalizedBlock(block: Block, dag: DagRepresentation[F]): F[Boolean] =
    Metrics[F].timer("updateLastFinalizedBlock") {
      for {
        lastFinalizedBlockHash <- LastFinalizedBlockHashContainer[F].get
        result <- FinalityDetectorVotingMatrix[F].onNewBlockAddedToTheBlockDag(
                   dag,
                   block,
                   lastFinalizedBlockHash
                 )
        changed <- result.fold(false.pure[F]) {
                    case CommitteeWithConsensusValue(validator, quorum, consensusValue) =>
                      Log[F].info(
                        s"New last finalized block hash is ${PrettyPrinter.buildString(consensusValue)}."
                      ) >>
                        LastFinalizedBlockHashContainer[F].set(consensusValue).as(true)
                  }
      } yield changed
    }

  /** Check that either we have the block already scheduled but missing dependencies, or it's in the store */
  def contains(
      block: Block
  ): F[Boolean] =
    BlockStorage[F].contains(block.blockHash)

  /** Return the list of tips. */
  def estimator(
      dag: DagRepresentation[F],
      lfbHash: BlockHash,
      latestMessagesHashes: Map[ByteString, Set[BlockHash]],
      equivocators: Set[Validator]
  ): F[NonEmptyList[BlockHash]] =
    Metrics[F].timer("estimator") {
      Estimator.tips[F](
        dag,
        lfbHash,
        latestMessagesHashes,
        equivocators
      )
    }

  /*
   * Logic:
   *  -Score each of the DAG heads extracted from the block messages via GHOST
   *  (Greedy Heaviest-Observed Sub-Tree)
   *  -Let P = subset of heads such that P contains no conflicts and the total score is maximized
   *  -Let R = subset of deploy messages which are not included in DAG obtained by following blocks in P
   *  -If R is non-empty then create a new block with parents equal to P and (non-conflicting) txns obtained from R
   *  -Else if R is empty and |P| > 1 then create a block with parents equal to P and no transactions
   *  -Else None
   *
   *  TODO: Make this return Either so that we get more information about why not block was
   *  produced (no deploys, already processing, no validator id)
   */
  def createMessage(canCreateBallot: Boolean): F[CreateBlockStatus] = validatorId match {
    case Some(ValidatorIdentity(publicKey, privateKey, sigAlgorithm)) =>
      Metrics[F].timer("createBlock") {
        for {
          dag                 <- dag
          latestMessages      <- dag.latestMessages
          latestMessageHashes = latestMessages.mapValues(_.map(_.messageHash))
          equivocators        <- dag.getEquivocators
          lfbHash             <- LastFinalizedBlockHashContainer[F].get
          // Tips can be either ballots or blocks.
          tipHashes <- estimator(dag, lfbHash, latestMessageHashes, equivocators)
          tips      <- tipHashes.traverse(ProtoUtil.unsafeGetBlock[F])
          _ <- Log[F].info(
                s"Estimates are ${tipHashes.toList.map(PrettyPrinter.buildString).mkString(", ") -> "tips"}"
              )
          _ <- Log[F].info(
                s"Fork-choice is ${PrettyPrinter.buildString(tipHashes.head) -> "block"}."
              )
          // Merged makes sure that we only get blocks.
          merged  <- ExecEngineUtil.merge[F](tips, dag).timer("mergeTipsEffects")
          parents = merged.parents
          _ <- Log[F].info(
                s"${parents.size} parents out of ${tipHashes.size} latest blocks will be used."
              )

          // Push any unfinalized deploys which are still in the buffer back to pending state if the
          // blocks they were contained have become orphans since we last tried to propose a block.
          // Doing this here rather than after adding blocks because it's quite costly; the downside
          // is that the auto-proposer will not pick up the change in the pending set immediately.
          requeued <- DeployBuffer.requeueOrphanedDeploys[F](merged.parents.map(_.blockHash).toSet)
          _        <- Log[F].info(s"Re-queued $requeued orphaned deploys.").whenA(requeued > 0)

          timestamp <- Time[F].currentMillis
          props     <- CreateMessageProps(publicKey, latestMessages, merged)
          remainingHashes <- DeployBuffer.remainingDeploys[F](
                              dag,
                              parents.map(_.blockHash).toSet,
                              timestamp,
                              props.configuration.deployConfig
                            )
          proposal <- if (remainingHashes.nonEmpty) {
                       createProposal(
                         dag,
                         props,
                         latestMessages,
                         merged,
                         remainingHashes,
                         publicKey,
                         privateKey,
                         sigAlgorithm,
                         timestamp,
                         lfbHash
                       )
                     } else if (canCreateBallot && merged.parents.nonEmpty) {
                       createBallot(
                         latestMessages,
                         merged,
                         publicKey,
                         privateKey,
                         sigAlgorithm,
                         lfbHash
                       )
                     } else {
                       CreateBlockStatus.noNewDeploys.pure[F]
                     }
          signedBlock = proposal match {
            case Created(block) =>
              Created(signBlock(block, privateKey, sigAlgorithm))
            case _ => proposal
          }
        } yield signedBlock
      }
    case None => CreateBlockStatus.readOnlyMode.pure[F]
  }

  def lastFinalizedBlock: F[Block] =
    for {
      lastFinalizedBlockHash <- LastFinalizedBlockHashContainer[F].get
      block                  <- ProtoUtil.unsafeGetBlock[F](lastFinalizedBlockHash)
    } yield block

  // Collection of props for creating blocks or ballots.
  private case class CreateMessageProps(
      justifications: Seq[Justification],
      parents: Seq[BlockHash],
      rank: Long,
      protocolVersion: ProtocolVersion,
      configuration: Config,
      validatorSeqNum: Int,
      validatorPrevBlockHash: ByteString
  )
  private object CreateMessageProps {
    def apply(
        validatorId: Keys.PublicKey,
        latestMessages: Map[ByteString, Set[Message]],
        merged: ExecEngineUtil.MergeResult[ExecEngineUtil.TransformMap, Block]
    ): F[CreateMessageProps] = {
      // We ensure that only the justifications given in the block are those
      // which are bonded validators in the chosen parent. This is safe because
      // any latest message not from a bonded validator will not change the
      // final fork-choice.
      val bondedValidators = bonds(merged.parents.head).map(_.validatorPublicKey).toSet
      val bondedLatestMsgs = latestMessages.filter { case (v, _) => bondedValidators.contains(v) }
      // TODO: Remove redundant justifications.
      val justifications = toJustification(latestMessages.values.flatten.toSeq)
      // Start numbering from 1 (validator's first block seqNum = 1)
      val latestMessage = latestMessages
        .get(ByteString.copyFrom(validatorId))
        .map(_.maxBy(_.validatorMsgSeqNum))
      val validatorSeqNum        = latestMessage.fold(1)(_.validatorMsgSeqNum + 1)
      val validatorPrevBlockHash = latestMessage.fold(ByteString.EMPTY)(_.messageHash)
      for {
        // `bondedLatestMsgs` won't include Genesis block
        // and in the case when it becomes the main parent we want to include its rank
        // when calculating it for the current block.
        rank <- MonadThrowable[F].fromTry(
                 merged.parents.toList
                   .traverse(Message.fromBlock(_))
                   .map(_.toSet | bondedLatestMsgs.values.flatten.toSet)
                   .map(set => ProtoUtil.nextRank(set.toList))
               )
        config          <- CasperLabsProtocol[F].configAt(rank)
        protocolVersion <- CasperLabsProtocol[F].versionAt(rank)
      } yield CreateMessageProps(
        justifications,
        merged.parents.map(_.blockHash).toSeq,
        rank,
        protocolVersion,
        config,
        validatorSeqNum,
        validatorPrevBlockHash
      )
    }
  }

  //TODO: Need to specify SEQ vs PAR type block?
  /** Execute a set of deploys in the context of chosen parents. Compile them into a block if everything goes fine. */
  private def createProposal(
      dag: DagRepresentation[F],
      props: CreateMessageProps,
      latestMessages: Map[ByteString, Set[Message]],
      merged: ExecEngineUtil.MergeResult[ExecEngineUtil.TransformMap, Block],
      remainingHashes: Set[ByteString],
      validatorId: Keys.PublicKey,
      privateKey: Keys.PrivateKey,
      sigAlgorithm: SignatureAlgorithm,
      timestamp: Long,
      lfbHash: BlockHash
  ): F[CreateBlockStatus] =
    Metrics[F].timer("createProposal") {
      val deployStream =
        DeployStorageReader[F]
          .getByHashes(remainingHashes)
          .through(
            DeployFilters.Pipes.dependenciesMet[F](dag, props.parents.toSet)
          )
      (for {
        checkpoint <- ExecEngineUtil
                       .computeDeploysCheckpoint[F](
                         merged,
                         deployStream,
                         timestamp,
                         props.protocolVersion,
                         props.rank,
                         upgrades
                       )
        result <- Sync[F]
                   .delay {
                     if (checkpoint.deploysForBlock.isEmpty) {
                       CreateBlockStatus.noNewDeploys
                     } else {
                       val block = ProtoUtil.block(
                         props.justifications,
                         checkpoint.preStateHash,
                         checkpoint.postStateHash,
                         checkpoint.bondedValidators,
                         checkpoint.deploysForBlock,
                         props.protocolVersion,
                         merged.parents.map(_.blockHash),
                         props.validatorSeqNum,
                         props.validatorPrevBlockHash,
                         chainName,
                         timestamp,
                         props.rank,
                         validatorId,
                         privateKey,
                         sigAlgorithm,
                         lfbHash
                       )
                       CreateBlockStatus.created(block)
                     }
                   }
                   .timer("blockInstance")
      } yield result).handleErrorWith {
        case ex @ SmartContractEngineError(error) =>
          Log[F]
            .error(
              s"Execution Engine returned an error while processing deploys: $error"
            )
            .as(CreateBlockStatus.internalDeployError(ex))
        case NonFatal(ex) =>
          Log[F]
            .error(
              s"Critical error encountered while processing deploys: $ex"
            )
            .as(CreateBlockStatus.internalDeployError(ex))
      }
    }

  private def createBallot(
      latestMessages: Map[ByteString, Set[Message]],
      merged: ExecEngineUtil.MergeResult[ExecEngineUtil.TransformMap, Block],
      validatorId: Keys.PublicKey,
      privateKey: Keys.PrivateKey,
      sigAlgorithm: SignatureAlgorithm,
      keyBlockHash: ByteString
  ): F[CreateBlockStatus] =
    for {
      now    <- Time[F].currentMillis
      props  <- CreateMessageProps(validatorId, latestMessages, merged)
      parent = merged.parents.head
      block = ProtoUtil.ballot(
        props.justifications,
        parent.getHeader.getState.postStateHash,
        parent.getHeader.getState.bonds,
        props.protocolVersion,
        parent.blockHash,
        props.validatorSeqNum,
        props.validatorPrevBlockHash,
        chainName,
        now,
        props.rank,
        validatorId,
        privateKey,
        sigAlgorithm,
        keyBlockHash
      )
    } yield CreateBlockStatus.created(block)

  // MultiParentCasper Exposes the block DAG to those who need it.
  def dag: F[DagRepresentation[F]] =
    DagStorage[F].getRepresentation

  /** Remove all the blocks that were successfully added from the block buffer and the dependency DAG. */
  private def removeAdded(
      attempts: List[(Block, BlockStatus)],
      canRemove: BlockStatus => Boolean
  ): F[Unit] = {
    val addedBlocks = attempts.collect {
      case (block, status) if canRemove(status) => block
    }.toList

    val addedBlockHashes = addedBlocks.map(_.blockHash)

    // Mark deploys we have observed in blocks as processed
    val processedDeploys =
      addedBlocks.flatMap(block => block.getBody.deploys.map(_.getDeploy)).toList

    DeployStorageWriter[F].markAsProcessed(processedDeploys) >>
      statelessExecutor.removeDependencies(addedBlockHashes)
  }

  /** The new gossiping first syncs the missing DAG, then downloads and adds the blocks in topological order.
    * However the EquivocationDetector wants to know about dependencies so it can assign different statuses,
    * so we'll make the synchronized DAG known via a partial block message, so any missing dependencies can
    * be tracked, i.e. Casper will know about the pending graph.  */
  def addMissingDependencies(block: Block): F[Unit] =
    statelessExecutor.addMissingDependencies(block)
}

object MultiParentCasperImpl {

  def create[F[_]: Concurrent: Log: Metrics: Time: BlockStorage: DagStorage: DeployBuffer: ExecutionEngineService: LastFinalizedBlockHashContainer: DeployStorage: Validation: CasperLabsProtocol: Cell[
    *[_],
    CasperState
  ]: DeploySelection](
      semaphoreMap: SemaphoreMap[F, ByteString],
      statelessExecutor: StatelessExecutor[F],
      validatorId: Option[ValidatorIdentity],
      genesis: Block,
      chainName: String,
      minTtl: FiniteDuration,
      upgrades: Seq[ipc.ChainSpec.UpgradePoint],
      faultToleranceThreshold: Double = 0.1
  ): F[MultiParentCasper[F]] =
    for {
      dag <- DagStorage[F].getRepresentation
      lmh <- dag.latestMessageHashes
      // A stopgap solution to initialize the Last Finalized Block while we don't have the finality streams
      // that we can use to mark every final block in the database and just look up the latest upon restart.
      lca <- NonEmptyList.fromList(lmh.values.flatten.toList).fold(genesis.blockHash.pure[F]) {
              hashes =>
                DagOperations.latestCommonAncestorsMainParent[F](dag, hashes).map(_.messageHash)
            }
      implicit0(finalizer: FinalityDetectorVotingMatrix[F]) <- FinalityDetectorVotingMatrix
                                                                .of[F](
                                                                  dag,
                                                                  lca,
                                                                  faultToleranceThreshold
                                                                )
      _ <- LastFinalizedBlockHashContainer[F].set(lca)
    } yield new MultiParentCasperImpl[F](
      semaphoreMap,
      statelessExecutor,
      validatorId,
      genesis,
      chainName,
      minTtl,
      upgrades
    )

  /** Component purely to validate, execute and store blocks.
    * Even the Genesis, to create it in the first place. */
  class StatelessExecutor[F[_]: MonadThrowable: Time: Log: BlockStorage: DagStorage: ExecutionEngineService: Metrics: DeployStorageWriter: Validation: LastFinalizedBlockHashContainer: CasperLabsProtocol: Fs2Compiler](
      validatorId: Option[Keys.PublicKey],
      chainName: String,
      upgrades: Seq[ipc.ChainSpec.UpgradePoint],
      // NOTE: There was a race condition where probably a block was being removed at the same time
      // as its child was scheduled. The parent was still not in the DAG database, so it was considered
      // a missing dependency. It got removed and immediately added back, which later caused an assertion failure.
      // This semaphore should protect against state mutations that should be serialised, including reads.
      semaphore: Semaphore[F]
  ) {
    //TODO pull out
    implicit val functorRaiseInvalidBlock = validation.raiseValidateErrorThroughApplicativeError[F]

    /* Execute the block to get the effects then do some more validation.
     * Save the block if everything checks out.
     * We want to catch equivocations only after we confirm that the block completing
     * the equivocation is otherwise valid. */
    def validateAndAddBlock(
        maybeContext: Option[StatelessExecutor.Context],
        block: Block
    )(implicit state: Cell[F, CasperState]): F[BlockStatus] = {
      import io.casperlabs.casper.validation.ValidationImpl.metricsSource
      Metrics[F].timer("validateAndAddBlock") {
        val hashPrefix = PrettyPrinter.buildString(block.blockHash)
        val validationStatus = (for {
          _   <- Log[F].info(s"Attempting to add ${hashPrefix -> "block"} to the DAG.")
          dag <- DagStorage[F].getRepresentation
          _ <- Validation[F].blockFull(
                block,
                dag,
                chainName,
                maybeContext.map(_.genesis)
              )
          casperState <- Cell[F, CasperState].read
          // Confirm the parents are correct (including checking they commute) and capture
          // the effect needed to compute the correct pre-state as well.
          _ <- Log[F].debug(s"Validating the parents of ${hashPrefix -> "block"}")
          merged <- maybeContext.fold(
                     ExecEngineUtil.MergeResult
                       .empty[ExecEngineUtil.TransformMap, Block]
                       .pure[F]
                   ) { _ =>
                     Validation[F].parents(block, dag)
                   }
          _ <- Log[F].debug(s"Computing the pre-state hash of ${hashPrefix -> "block"}")
          preStateHash <- ExecEngineUtil
                           .computePrestate[F](merged, block.getHeader.rank, upgrades)
                           .timer("computePrestate")
          _ <- Log[F].debug(s"Computing the effects for ${hashPrefix -> "block"}")
          blockEffects <- ExecEngineUtil
                           .effectsForBlock[F](block, preStateHash)
                           .recoverWith {
                             case NonFatal(ex) =>
                               Log[F].error(
                                 s"Could not calculate effects for ${hashPrefix -> "block"}: $ex"
                               ) *>
                                 FunctorRaise[F, InvalidBlock].raise(InvalidTransaction)
                           }
                           .timer("effectsForBlock")
          gasSpent = block.getBody.deploys.foldLeft(0L) { case (acc, next) => acc + next.cost }
          _ <- Metrics[F]
                .incrementCounter("gas_spent", gasSpent)
          _ <- Log[F].debug(s"Validating the transactions in ${hashPrefix -> "block"}")
          _ <- Validation[F].transactions(
                block,
                preStateHash,
                blockEffects
              )
          _ <- Log[F].debug(s"Validating neglection for ${hashPrefix -> "block"}")
          _ <- Validation[F]
                .neglectedInvalidBlock(
                  block,
                  casperState.invalidBlockTracker
                )
          _ <- Log[F].debug(s"Checking equivocation for ${hashPrefix -> "block"}")
          _ <- EquivocationDetector
                .checkEquivocationWithUpdate[F](dag, block)
                .timer("checkEquivocationsWithUpdate")
          _ <- Log[F].debug(s"Block effects calculated for ${hashPrefix -> "block"}")
        } yield blockEffects).attempt

        validationStatus.flatMap {
          case Right(effects) =>
            addEffects(Valid, block, effects).as(Valid)

          case Left(DropErrorWrapper(invalid)) =>
            // These exceptions are coming from the validation checks that used to happen outside attemptAdd,
            // the ones that returned boolean values.
            (invalid: BlockStatus).pure[F]

          case Left(ValidateErrorWrapper(EquivocatedBlock))
              if validatorId.map(ByteString.copyFrom).exists(_ == block.validatorPublicKey) =>
            addEffects(SelfEquivocatedBlock, block, Seq.empty).as(SelfEquivocatedBlock)

          case Left(ValidateErrorWrapper(invalid)) =>
            addEffects(invalid, block, Seq.empty).as(invalid)

          case Left(ex) =>
            for {
              _ <- Log[F].error(
                    s"Unexpected exception during validation of ${PrettyPrinter
                      .buildString(block.blockHash) -> "block"}: $ex"
                  )
              _ <- ex.raiseError[F, BlockStatus]
            } yield UnexpectedBlockException(ex)
        }
      }
    }

    // TODO: Handle slashing
    /** Either store the block with its transformation,
      * or add it to the buffer in case the dependencies are missing. */
    def addEffects(
        status: BlockStatus,
        block: Block,
        transforms: Seq[ipc.TransformEntry]
    )(implicit state: Cell[F, CasperState]): F[Unit] =
      status match {
        //Add successful! Send block to peers, log success, try to add other blocks
        case Valid =>
          for {
            _ <- addToState(block, transforms)
            _ <- Log[F].info(
                  s"Added ${PrettyPrinter.buildString(block.blockHash) -> "block"}"
                )
          } yield ()

        case MissingBlocks =>
          addMissingDependencies(block)

        case EquivocatedBlock | SelfEquivocatedBlock =>
          for {
            _ <- addToState(block, transforms)
            _ <- Log[F].info(
                  s"Added equivocated ${PrettyPrinter.buildString(block.blockHash) -> "block"}"
                )
          } yield ()

        case InvalidUnslashableBlock | InvalidBlockNumber | InvalidParents | InvalidSequenceNumber |
            InvalidPrevBlockHash | NeglectedInvalidBlock | InvalidTransaction | InvalidBondsCache |
            InvalidRepeatDeploy | InvalidChainName | InvalidBlockHash | InvalidDeployCount |
            InvalidDeployHash | InvalidDeploySignature | InvalidPreStateHash |
            InvalidPostStateHash | InvalidTargetHash | InvalidDeployHeader |
            InvalidDeployChainName | DeployDependencyNotMet | DeployExpired | DeployFromFuture |
            SwimlaneMerged =>
          handleInvalidBlockEffect(status, block)

        case Processing | Processed =>
          throw new RuntimeException(s"A block should not be processing at this stage.")

        case UnexpectedBlockException(ex) =>
          Log[F].error(s"Encountered exception in while processing ${PrettyPrinter
            .buildString(block.blockHash) -> "block"}: $ex")
      }

    /** Remember a block as being invalid, then save it to storage. */
    private def handleInvalidBlockEffect(
        status: BlockStatus,
        block: Block
    )(implicit state: Cell[F, CasperState]): F[Unit] =
      for {
        _ <- Log[F].warn(
              s"Recording invalid ${PrettyPrinter.buildString(block.blockHash) -> "block"} for $status."
            )
        // TODO: Slash block for status except InvalidUnslashableBlock
        // TODO: Persist invalidBlockTracker into Dag
        _ <- Cell[F, CasperState].modify { s =>
              s.copy(invalidBlockTracker = s.invalidBlockTracker + block.blockHash)
            }
      } yield ()

    /** Save the block to the block and DAG storage. */
    private def addToState(
        block: Block,
        effects: Seq[ipc.TransformEntry]
    ): F[Unit] =
      semaphore.withPermit {
        BlockStorage[F].put(block.blockHash, BlockMsgWithTransform(Some(block), effects))
      }

    /** Check if the block has dependencies that we don't have in store.
      * Add those to the dependency DAG. */
    def addMissingDependencies(
        block: Block
    )(implicit state: Cell[F, CasperState]): F[Unit] =
      semaphore.withPermit {
        for {
          dag                 <- DagStorage[F].getRepresentation
          missingDependencies <- dependenciesHashesOf(block).filterA(dag.contains(_).map(!_))
          _ <- Cell[F, CasperState].modify { s =>
                s.copy(
                  dependencyDag = missingDependencies.foldLeft(s.dependencyDag) {
                    case (deps, ancestorHash) => deps.add(ancestorHash, block.blockHash)
                  }
                )
              }
        } yield ()
      }

    /** Remove block relationships we scheduled earlier after blocks have been added. */
    def removeDependencies(
        addedBlocksHashes: List[ByteString]
    )(implicit state: Cell[F, CasperState]): F[Unit] =
      semaphore.withPermit {
        Cell[F, CasperState].modify { s =>
          s.copy(
            dependencyDag = addedBlocksHashes.foldLeft(s.dependencyDag) {
              case (dag, blockHash) =>
                dag.remove(blockHash)
            }
          )
        }
      }
  }

  object StatelessExecutor {
    case class Context(genesis: Block, lastFinalizedBlockHash: BlockHash)

    def establishMetrics[F[_]: Metrics]: F[Unit] = {
      implicit val src = CasperMetricsSource
      Metrics[F].incrementCounter("gas_spent", 0L)
    }

    def create[F[_]: Concurrent: Time: Log: BlockStorage: DagStorage: ExecutionEngineService: Metrics: DeployStorage: Validation: LastFinalizedBlockHashContainer: CasperLabsProtocol: Fs2Compiler](
        validatorId: Option[Keys.PublicKey],
        chainName: String,
        upgrades: Seq[ipc.ChainSpec.UpgradePoint]
    ): F[StatelessExecutor[F]] =
      for {
        _ <- establishMetrics[F]
        s <- Semaphore[F](1)
      } yield new StatelessExecutor[F](validatorId, chainName, upgrades, s)
  }

  /** Encapsulating all methods that might use peer-to-peer communication. */
  @typeclass trait Broadcaster[F[_]] {
    def networkEffects(
        block: Block,
        status: BlockStatus
    ): F[Unit]
  }

  object Broadcaster {

    /** Network access using the new RPC style gossiping. */
    def fromGossipServices[F[_]: Applicative](
        validatorId: Option[ValidatorIdentity],
        relaying: gossiping.Relaying[F]
    ): Broadcaster[F] = new Broadcaster[F] {

      private val maybeOwnPublickKey = validatorId map {
        case ValidatorIdentity(publicKey, _, _) =>
          ByteString.copyFrom(publicKey)
      }

      def networkEffects(
          block: Block,
          status: BlockStatus
      ): F[Unit] =
        status match {
          case Valid | EquivocatedBlock =>
            maybeOwnPublickKey match {
              case Some(key) if key == block.getHeader.validatorPublicKey =>
                relaying.relay(List(block.blockHash)).void
              case _ =>
                // We were adding somebody else's block. The DownloadManager did the gossiping.
                ().pure[F]
            }

          case SelfEquivocatedBlock =>
            // Don't relay block with which a node has equivocated.
            ().pure[F]

          case MissingBlocks =>
            throw new RuntimeException(
              "Impossible! The DownloadManager should not tell Casper about blocks with missing dependencies!"
            )

          case InvalidUnslashableBlock | InvalidBlockNumber | InvalidParents |
              InvalidSequenceNumber | InvalidPrevBlockHash | NeglectedInvalidBlock |
              InvalidTransaction | InvalidBondsCache | InvalidChainName | InvalidBlockHash |
              InvalidDeployCount | InvalidPreStateHash | InvalidPostStateHash | SwimlaneMerged |
              InvalidTargetHash | Processing | Processed =>
            ().pure[F]

          case InvalidRepeatDeploy | InvalidChainName | InvalidDeployHash | InvalidDeploySignature |
              InvalidDeployChainName | InvalidDeployHeader | DeployDependencyNotMet |
              DeployExpired =>
            ().pure[F]

          case UnexpectedBlockException(_) | DeployFromFuture =>
            ().pure[F]
        }
    }

    def noop[F[_]: Applicative]: Broadcaster[F] = new Broadcaster[F] {
      override def networkEffects(block: Block, status: BlockStatus): F[Unit] = Applicative[F].unit
    }
  }

}
