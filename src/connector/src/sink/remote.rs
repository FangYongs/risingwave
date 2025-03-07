// Copyright 2024 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::pin;
use std::time::Instant;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::future::select;
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use jni::JavaVM;
use prost::Message;
use risingwave_common::array::StreamChunk;
use risingwave_common::catalog::{ColumnDesc, ColumnId};
use risingwave_common::error::anyhow_error;
use risingwave_common::types::DataType;
use risingwave_jni_core::jvm_runtime::JVM;
use risingwave_jni_core::{
    call_static_method, gen_class_name, JniReceiverType, JniSenderType, JniSinkWriterStreamRequest,
};
use risingwave_pb::connector_service::sink_coordinator_stream_request::StartCoordinator;
use risingwave_pb::connector_service::sink_writer_stream_request::{
    Request as SinkRequest, StartSink,
};
use risingwave_pb::connector_service::{
    sink_coordinator_stream_request, sink_coordinator_stream_response, sink_writer_stream_response,
    PbSinkParam, SinkCoordinatorStreamRequest, SinkCoordinatorStreamResponse, SinkMetadata,
    SinkPayloadFormat, SinkWriterStreamRequest, SinkWriterStreamResponse, TableSchema,
    ValidateSinkRequest, ValidateSinkResponse,
};
use risingwave_rpc_client::error::RpcError;
use risingwave_rpc_client::{
    BidiStreamReceiver, BidiStreamSender, SinkCoordinatorStreamHandle, SinkWriterStreamHandle,
    DEFAULT_BUFFER_SIZE,
};
use rw_futures_util::drop_either_future;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{unbounded_channel, Receiver, Sender};
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

use super::elasticsearch::{StreamChunkConverter, ES_OPTION_DELIMITER};
use crate::sink::catalog::desc::SinkDesc;
use crate::sink::coordinate::CoordinatedSinkWriter;
use crate::sink::log_store::{LogStoreReadItem, TruncateOffset};
use crate::sink::writer::{LogSinkerOf, SinkWriter, SinkWriterExt};
use crate::sink::{
    DummySinkCommitCoordinator, LogSinker, Result, Sink, SinkCommitCoordinator, SinkError,
    SinkLogReader, SinkMetrics, SinkParam, SinkWriterParam,
};
use crate::ConnectorParams;

macro_rules! def_remote_sink {
    () => {
        def_remote_sink! {
            { ElasticSearch, ElasticSearchSink, "elasticsearch" }
            { Cassandra, CassandraSink, "cassandra" }
            { Jdbc, JdbcSink, "jdbc", |desc| {
                desc.sink_type.is_append_only()
            } }
            { DeltaLake, DeltaLakeSink, "deltalake" }
            { HttpJava, HttpJavaSink, "http" }
        }
    };
    () => {};
    ({ $variant_name:ident, $sink_type_name:ident, $sink_name:expr }) => {
        #[derive(Debug)]
        pub struct $variant_name;
        impl RemoteSinkTrait for $variant_name {
            const SINK_NAME: &'static str = $sink_name;
        }
        pub type $sink_type_name = RemoteSink<$variant_name>;
    };
    ({ $variant_name:ident, $sink_type_name:ident, $sink_name:expr, |$desc:ident| $body:expr }) => {
        #[derive(Debug)]
        pub struct $variant_name;
        impl RemoteSinkTrait for $variant_name {
            const SINK_NAME: &'static str = $sink_name;
            fn default_sink_decouple($desc: &SinkDesc) -> bool {
                $body
            }
        }
        pub type $sink_type_name = RemoteSink<$variant_name>;
    };
    ({ $($first:tt)+ } $({$($rest:tt)+})*) => {
        def_remote_sink! {
            {$($first)+}
        }
        def_remote_sink! {
            $({$($rest)+})*
        }
    };
    ($($invalid:tt)*) => {
        compile_error! {concat! {"invalid `", stringify!{$($invalid)*}, "`"}}
    }
}

def_remote_sink!();

pub trait RemoteSinkTrait: Send + Sync + 'static {
    const SINK_NAME: &'static str;
    fn default_sink_decouple(_desc: &SinkDesc) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct RemoteSink<R: RemoteSinkTrait> {
    param: SinkParam,
    _phantom: PhantomData<R>,
}

impl<R: RemoteSinkTrait> TryFrom<SinkParam> for RemoteSink<R> {
    type Error = SinkError;

    fn try_from(param: SinkParam) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            param,
            _phantom: PhantomData,
        })
    }
}

impl<R: RemoteSinkTrait> Sink for RemoteSink<R> {
    type Coordinator = DummySinkCommitCoordinator;
    type LogSinker = RemoteLogSinker;

    const SINK_NAME: &'static str = R::SINK_NAME;

    fn default_sink_decouple(desc: &SinkDesc) -> bool {
        R::default_sink_decouple(desc)
    }

    async fn new_log_sinker(&self, writer_param: SinkWriterParam) -> Result<Self::LogSinker> {
        RemoteLogSinker::new(self.param.clone(), writer_param, Self::SINK_NAME).await
    }

    async fn validate(&self) -> Result<()> {
        validate_remote_sink(&self.param, Self::SINK_NAME).await?;
        Ok(())
    }
}

async fn validate_remote_sink(param: &SinkParam, sink_name: &str) -> anyhow::Result<()> {
    if sink_name == ElasticSearchSink::SINK_NAME
        && param.downstream_pk.len() > 1
        && param.properties.get(ES_OPTION_DELIMITER).is_none()
    {
        return Err(anyhow_error!(
            "Es sink only support single pk or pk with delimiter option"
        ));
    }
    // FIXME: support struct and array in stream sink
    param.columns.iter().map(|col| {
        match &col.data_type {
            DataType::Int16
                    | DataType::Int32
                    | DataType::Int64
                    | DataType::Float32
                    | DataType::Float64
                    | DataType::Boolean
                    | DataType::Decimal
                    | DataType::Timestamp
                    | DataType::Timestamptz
                    | DataType::Varchar
                    | DataType::Date
                    | DataType::Time
                    | DataType::Interval
                    | DataType::Jsonb
                    | DataType::Bytea => Ok(()),
            DataType::List(list) => {
                if (sink_name==ElasticSearchSink::SINK_NAME) | matches!(list.as_ref(), DataType::Int16 | DataType::Int32 | DataType::Int64 | DataType::Float32 | DataType::Float64 | DataType::Varchar){
                    Ok(())
                } else{
                    Err(SinkError::Remote(anyhow_error!(
                        "Remote sink only support list<int16, int32, int64, float, double, varchar>, got {:?}: {:?}",
                        col.name,
                        col.data_type,
                    )))
                }
            },
            DataType::Struct(_) => {
                if sink_name==ElasticSearchSink::SINK_NAME{
                    Ok(())
                }else{
                    Err(SinkError::Remote(anyhow_error!(
                        "Only Es sink support struct, got {:?}: {:?}",
                        col.name,
                        col.data_type,
                    )))
                }
            },
            DataType::Serial | DataType::Int256 => Err(SinkError::Remote(anyhow_error!(
                            "remote sink supports Int16, Int32, Int64, Float32, Float64, Boolean, Decimal, Time, Date, Interval, Jsonb, Timestamp, Timestamptz, Bytea, List and Varchar, (Es sink support Struct) got {:?}: {:?}",
                            col.name,
                            col.data_type,
                        )))}}).try_collect()?;

    let jvm = JVM.get_or_init()?;
    let sink_param = param.to_proto();

    spawn_blocking(move || {
        let mut env = jvm.attach_current_thread()?;
        let validate_sink_request = ValidateSinkRequest {
            sink_param: Some(sink_param),
        };
        let validate_sink_request_bytes =
            env.byte_array_from_slice(&Message::encode_to_vec(&validate_sink_request))?;

        let validate_sink_response_bytes = call_static_method!(
            env,
            {com.risingwave.connector.JniSinkValidationHandler},
            {byte[] validate(byte[] validateSourceRequestBytes)},
            &validate_sink_request_bytes
        )?;

        let validate_sink_response: ValidateSinkResponse = Message::decode(
            risingwave_jni_core::to_guarded_slice(&validate_sink_response_bytes, &mut env)?.deref(),
        )?;

        validate_sink_response.error.map_or_else(
            || Ok(()), // If there is no error message, return Ok here.
            |err| {
                Err(anyhow!(format!(
                    "sink cannot pass validation: {}",
                    err.error_message
                )))
            },
        )
    })
    .await
    .context("JoinHandle returns error")?
}

pub struct RemoteLogSinker {
    request_sender: BidiStreamSender<JniSinkWriterStreamRequest>,
    response_stream: BidiStreamReceiver<SinkWriterStreamResponse>,
    sink_metrics: SinkMetrics,
    stream_chunk_converter: StreamChunkConverter,
}

impl RemoteLogSinker {
    async fn new(
        sink_param: SinkParam,
        writer_param: SinkWriterParam,
        sink_name: &str,
    ) -> Result<Self> {
        let sink_proto = sink_param.to_proto();
        let payload_schema = if sink_name == ElasticSearchSink::SINK_NAME {
            let columns = vec![
                ColumnDesc::unnamed(ColumnId::from(0), DataType::Varchar).to_protobuf(),
                ColumnDesc::unnamed(ColumnId::from(1), DataType::Jsonb).to_protobuf(),
            ];
            Some(TableSchema {
                columns,
                pk_indices: vec![],
            })
        } else {
            sink_proto.table_schema.clone()
        };

        let SinkWriterStreamHandle {
            request_sender,
            response_stream,
        } = EmbeddedConnectorClient::new()?
            .start_sink_writer_stream(payload_schema, sink_proto, SinkPayloadFormat::StreamChunk)
            .await?;

        let sink_metrics = writer_param.sink_metrics;
        Ok(RemoteLogSinker {
            request_sender,
            response_stream,
            sink_metrics,
            stream_chunk_converter: StreamChunkConverter::new(
                sink_name,
                sink_param.schema(),
                &sink_param.downstream_pk,
                &sink_param.properties,
            )?,
        })
    }
}

#[async_trait]
impl LogSinker for RemoteLogSinker {
    async fn consume_log_and_sink(self, log_reader: &mut impl SinkLogReader) -> Result<()> {
        let mut request_tx = self.request_sender;
        let mut response_err_stream_rx = self.response_stream;
        let sink_metrics = self.sink_metrics;

        let (response_tx, mut response_rx) = unbounded_channel();

        let poll_response_stream = async move {
            loop {
                let result = response_err_stream_rx.stream.try_next().await;
                match result {
                    Ok(Some(response)) => {
                        response_tx.send(response).map_err(|err| {
                            SinkError::Remote(anyhow!("unable to send response: {:?}", err.0))
                        })?;
                    }
                    Ok(None) => return Err(SinkError::Remote(anyhow!("end of response stream"))),
                    Err(e) => return Err(SinkError::Remote(anyhow!(e))),
                }
            }
        };

        let poll_consume_log_and_sink = async move {
            async fn truncate_matched_offset(
                queue: &mut VecDeque<(TruncateOffset, Option<Instant>)>,
                persisted_offset: TruncateOffset,
                log_reader: &mut impl SinkLogReader,
                metrics: &SinkMetrics,
            ) -> Result<()> {
                while let Some((sent_offset, _)) = queue.front()
                    && sent_offset < &persisted_offset
                {
                    queue.pop_front();
                }

                let (sent_offset, start_time) = queue.pop_front().ok_or_else(|| {
                    anyhow!("get unsent offset {:?} in response", persisted_offset)
                })?;
                if sent_offset != persisted_offset {
                    return Err(anyhow!(
                        "new response offset {:?} not match the buffer offset {:?}",
                        persisted_offset,
                        sent_offset
                    )
                    .into());
                }

                if let (TruncateOffset::Barrier { .. }, Some(start_time)) =
                    (persisted_offset, start_time)
                {
                    metrics
                        .sink_commit_duration_metrics
                        .observe(start_time.elapsed().as_millis() as f64);
                }

                log_reader.truncate(persisted_offset).await?;
                Ok(())
            }

            let mut prev_offset: Option<TruncateOffset> = None;
            // Push from back and pop from front
            let mut sent_offset_queue: VecDeque<(TruncateOffset, Option<Instant>)> =
                VecDeque::new();

            loop {
                let either_result: futures::future::Either<
                    Option<SinkWriterStreamResponse>,
                    anyhow::Result<(u64, LogStoreReadItem)>,
                > = drop_either_future(
                    select(pin!(response_rx.recv()), pin!(log_reader.next_item())).await,
                );
                match either_result {
                    futures::future::Either::Left(opt) => {
                        let response = opt.ok_or_else(|| anyhow!("end of response stream"))?;
                        match response {
                            SinkWriterStreamResponse {
                                response:
                                    Some(sink_writer_stream_response::Response::Batch(
                                        sink_writer_stream_response::BatchWrittenResponse {
                                            epoch,
                                            batch_id,
                                        },
                                    )),
                            } => {
                                truncate_matched_offset(
                                    &mut sent_offset_queue,
                                    TruncateOffset::Chunk {
                                        epoch,
                                        chunk_id: batch_id as _,
                                    },
                                    log_reader,
                                    &sink_metrics,
                                )
                                .await?;
                            }
                            SinkWriterStreamResponse {
                                response:
                                    Some(sink_writer_stream_response::Response::Commit(
                                        sink_writer_stream_response::CommitResponse {
                                            epoch,
                                            metadata,
                                        },
                                    )),
                            } => {
                                if let Some(metadata) = metadata {
                                    warn!("get unexpected non-empty metadata: {:?}", metadata);
                                }
                                truncate_matched_offset(
                                    &mut sent_offset_queue,
                                    TruncateOffset::Barrier { epoch },
                                    log_reader,
                                    &sink_metrics,
                                )
                                .await?;
                            }
                            response => {
                                return Err(SinkError::Remote(anyhow!(
                                    "get unexpected response: {:?}",
                                    response
                                )));
                            }
                        }
                    }
                    futures::future::Either::Right(result) => {
                        let (epoch, item): (u64, LogStoreReadItem) = result?;

                        match item {
                            LogStoreReadItem::StreamChunk { chunk, chunk_id } => {
                                let offset = TruncateOffset::Chunk { epoch, chunk_id };
                                if let Some(prev_offset) = &prev_offset {
                                    prev_offset.check_next_offset(offset)?;
                                }
                                let cardinality = chunk.cardinality();
                                sink_metrics
                                    .connector_sink_rows_received
                                    .inc_by(cardinality as _);

                                let chunk = self.stream_chunk_converter.convert_chunk(chunk)?;
                                request_tx
                                    .send_request(JniSinkWriterStreamRequest::Chunk {
                                        epoch,
                                        batch_id: chunk_id as u64,
                                        chunk,
                                    })
                                    .await?;
                                prev_offset = Some(offset);
                                sent_offset_queue
                                    .push_back((TruncateOffset::Chunk { epoch, chunk_id }, None));
                            }
                            LogStoreReadItem::Barrier { is_checkpoint } => {
                                let offset = TruncateOffset::Barrier { epoch };
                                if let Some(prev_offset) = &prev_offset {
                                    prev_offset.check_next_offset(offset)?;
                                }
                                let start_time = if is_checkpoint {
                                    let start_time = Instant::now();
                                    request_tx.barrier(epoch, true).await?;
                                    Some(start_time)
                                } else {
                                    request_tx.barrier(epoch, false).await?;
                                    None
                                };
                                prev_offset = Some(offset);
                                sent_offset_queue
                                    .push_back((TruncateOffset::Barrier { epoch }, start_time));
                            }
                            LogStoreReadItem::UpdateVnodeBitmap(_) => {}
                        }
                    }
                }
            }
        };

        select(pin!(poll_response_stream), pin!(poll_consume_log_and_sink))
            .await
            .factor_first()
            .0
    }
}

#[derive(Debug)]
pub struct CoordinatedRemoteSink<R: RemoteSinkTrait> {
    param: SinkParam,
    _phantom: PhantomData<R>,
}

impl<R: RemoteSinkTrait> TryFrom<SinkParam> for CoordinatedRemoteSink<R> {
    type Error = SinkError;

    fn try_from(param: SinkParam) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            param,
            _phantom: PhantomData,
        })
    }
}

impl<R: RemoteSinkTrait> Sink for CoordinatedRemoteSink<R> {
    type Coordinator = RemoteCoordinator;
    type LogSinker = LogSinkerOf<CoordinatedSinkWriter<CoordinatedRemoteSinkWriter>>;

    const SINK_NAME: &'static str = R::SINK_NAME;

    async fn validate(&self) -> Result<()> {
        validate_remote_sink(&self.param, Self::SINK_NAME).await?;
        Ok(())
    }

    async fn new_log_sinker(&self, writer_param: SinkWriterParam) -> Result<Self::LogSinker> {
        Ok(CoordinatedSinkWriter::new(
            writer_param
                .meta_client
                .expect("should have meta client")
                .sink_coordinate_client()
                .await,
            self.param.clone(),
            writer_param.vnode_bitmap.ok_or_else(|| {
                SinkError::Remote(anyhow_error!(
                    "sink needs coordination should not have singleton input"
                ))
            })?,
            CoordinatedRemoteSinkWriter::new(
                self.param.clone(),
                writer_param.connector_params,
                writer_param.sink_metrics.clone(),
            )
            .await?,
        )
        .await?
        .into_log_sinker(writer_param.sink_metrics))
    }

    async fn new_coordinator(&self) -> Result<Self::Coordinator> {
        RemoteCoordinator::new::<R>(self.param.clone()).await
    }
}

pub struct CoordinatedRemoteSinkWriter {
    properties: HashMap<String, String>,
    epoch: Option<u64>,
    batch_id: u64,
    stream_handle: SinkWriterStreamHandle<JniSinkWriterStreamRequest>,
    sink_metrics: SinkMetrics,
}

impl CoordinatedRemoteSinkWriter {
    pub async fn new(
        param: SinkParam,
        connector_params: ConnectorParams,
        sink_metrics: SinkMetrics,
    ) -> Result<Self> {
        let sink_proto = param.to_proto();
        let stream_handle = EmbeddedConnectorClient::new()?
            .start_sink_writer_stream(
                sink_proto.table_schema.clone(),
                sink_proto,
                connector_params.sink_payload_format,
            )
            .await?;

        Ok(Self {
            properties: param.properties,
            epoch: None,
            batch_id: 0,
            stream_handle,
            sink_metrics,
        })
    }

    fn for_test(
        response_receiver: Receiver<anyhow::Result<SinkWriterStreamResponse>>,
        request_sender: Sender<JniSinkWriterStreamRequest>,
    ) -> CoordinatedRemoteSinkWriter {
        let properties = HashMap::from([("output.path".to_string(), "/tmp/rw".to_string())]);

        let stream_handle = SinkWriterStreamHandle::for_test(
            request_sender,
            ReceiverStream::new(response_receiver)
                .map_err(RpcError::from)
                .boxed(),
        );

        CoordinatedRemoteSinkWriter {
            properties,
            epoch: None,
            batch_id: 0,
            stream_handle,
            sink_metrics: SinkMetrics::for_test(),
        }
    }
}

#[async_trait]
impl SinkWriter for CoordinatedRemoteSinkWriter {
    type CommitMetadata = Option<SinkMetadata>;

    async fn write_batch(&mut self, chunk: StreamChunk) -> Result<()> {
        let cardinality = chunk.cardinality();
        self.sink_metrics
            .connector_sink_rows_received
            .inc_by(cardinality as _);

        let epoch = self.epoch.ok_or_else(|| {
            SinkError::Remote(anyhow_error!(
                "epoch has not been initialize, call `begin_epoch`"
            ))
        })?;
        let batch_id = self.batch_id;
        self.stream_handle
            .request_sender
            .send_request(JniSinkWriterStreamRequest::Chunk {
                chunk,
                epoch,
                batch_id,
            })
            .await?;
        self.batch_id += 1;
        Ok(())
    }

    async fn begin_epoch(&mut self, epoch: u64) -> Result<()> {
        self.epoch = Some(epoch);
        Ok(())
    }

    async fn barrier(&mut self, is_checkpoint: bool) -> Result<Option<SinkMetadata>> {
        let epoch = self.epoch.ok_or_else(|| {
            SinkError::Remote(anyhow_error!(
                "epoch has not been initialize, call `begin_epoch`"
            ))
        })?;
        if is_checkpoint {
            // TODO: add metrics to measure commit time
            let rsp = self.stream_handle.commit(epoch).await?;
            rsp.metadata
                .ok_or_else(|| {
                    SinkError::Remote(anyhow_error!(
                        "get none metadata in commit response for coordinated sink writer"
                    ))
                })
                .map(Some)
        } else {
            self.stream_handle.barrier(epoch).await?;
            Ok(None)
        }
    }
}

pub struct RemoteCoordinator {
    stream_handle: SinkCoordinatorStreamHandle,
}

impl RemoteCoordinator {
    pub async fn new<R: RemoteSinkTrait>(param: SinkParam) -> Result<Self> {
        let stream_handle = EmbeddedConnectorClient::new()?
            .start_sink_coordinator_stream(param.clone())
            .await?;

        tracing::trace!(
            "{:?} RemoteCoordinator started with properties: {:?}",
            R::SINK_NAME,
            &param.properties
        );

        Ok(RemoteCoordinator { stream_handle })
    }
}

#[async_trait]
impl SinkCommitCoordinator for RemoteCoordinator {
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    async fn commit(&mut self, epoch: u64, metadata: Vec<SinkMetadata>) -> Result<()> {
        Ok(self.stream_handle.commit(epoch, metadata).await?)
    }
}

struct EmbeddedConnectorClient {
    jvm: &'static JavaVM,
}

impl EmbeddedConnectorClient {
    fn new() -> Result<Self> {
        let jvm = JVM
            .get_or_init()
            .context("failed to create EmbeddedConnectorClient")?;
        Ok(EmbeddedConnectorClient { jvm })
    }

    async fn start_sink_writer_stream(
        &self,
        payload_schema: Option<TableSchema>,
        sink_proto: PbSinkParam,
        sink_payload_format: SinkPayloadFormat,
    ) -> Result<SinkWriterStreamHandle<JniSinkWriterStreamRequest>> {
        let (handle, first_rsp) = SinkWriterStreamHandle::initialize(
            SinkWriterStreamRequest {
                request: Some(SinkRequest::Start(StartSink {
                    sink_param: Some(sink_proto),
                    format: sink_payload_format as i32,
                    payload_schema,
                })),
            },
            |rx| async move {
                let rx = self.start_jvm_worker_thread(
                    gen_class_name!(com.risingwave.connector.JniSinkWriterHandler),
                    "runJniSinkWriterThread",
                    rx,
                );
                Ok(ReceiverStream::new(rx).map_err(RpcError::from))
            },
        )
        .await?;

        match first_rsp {
            SinkWriterStreamResponse {
                response: Some(sink_writer_stream_response::Response::Start(_)),
            } => Ok(handle),
            msg => Err(SinkError::Internal(anyhow!(
                "should get start response but get {:?}",
                msg
            ))),
        }
    }

    async fn start_sink_coordinator_stream(
        &self,
        param: SinkParam,
    ) -> Result<SinkCoordinatorStreamHandle> {
        let (handle, first_rsp) = SinkCoordinatorStreamHandle::initialize(
            SinkCoordinatorStreamRequest {
                request: Some(sink_coordinator_stream_request::Request::Start(
                    StartCoordinator {
                        param: Some(param.to_proto()),
                    },
                )),
            },
            |rx| async move {
                let rx = self.start_jvm_worker_thread(
                    gen_class_name!(com.risingwave.connector.JniSinkCoordinatorHandler),
                    "runJniSinkCoordinatorThread",
                    rx,
                );
                Ok(ReceiverStream::new(rx).map_err(RpcError::from))
            },
        )
        .await?;

        match first_rsp {
            SinkCoordinatorStreamResponse {
                response: Some(sink_coordinator_stream_response::Response::Start(_)),
            } => Ok(handle),
            msg => Err(SinkError::Internal(anyhow!(
                "should get start response but get {:?}",
                msg
            ))),
        }
    }

    fn start_jvm_worker_thread<REQ: Send + 'static, RSP: Send + 'static>(
        &self,
        class_name: &'static str,
        method_name: &'static str,
        mut request_rx: JniReceiverType<REQ>,
    ) -> Receiver<std::result::Result<RSP, anyhow::Error>> {
        let (mut response_tx, response_rx): (JniSenderType<RSP>, _) =
            mpsc::channel(DEFAULT_BUFFER_SIZE);

        let jvm = self.jvm;
        std::thread::spawn(move || {
            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(e) => {
                    let _ = response_tx
                        .blocking_send(Err(anyhow!("failed to attach current thread: {:?}", e)));
                    return;
                }
            };

            let result = call_static_method!(
                env,
                class_name,
                method_name,
                {{void}, {long requestRx, long responseTx}},
                &mut request_rx as *mut JniReceiverType<REQ>,
                &mut response_tx as *mut JniSenderType<RSP>
            );

            match result {
                Ok(_) => {
                    tracing::info!("end of jni call {}::{}", class_name, method_name);
                }
                Err(e) => {
                    tracing::error!("jni call error: {:?}", e);
                }
            };
        });
        response_rx
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use risingwave_common::array::StreamChunk;
    use risingwave_common::test_prelude::StreamChunkTestExt;
    use risingwave_jni_core::JniSinkWriterStreamRequest;
    use risingwave_pb::connector_service::sink_writer_stream_request::{Barrier, Request};
    use risingwave_pb::connector_service::sink_writer_stream_response::{CommitResponse, Response};
    use risingwave_pb::connector_service::{SinkWriterStreamRequest, SinkWriterStreamResponse};
    use tokio::sync::mpsc;

    use crate::sink::remote::CoordinatedRemoteSinkWriter;
    use crate::sink::SinkWriter;

    #[tokio::test]
    async fn test_epoch_check() {
        let (request_sender, mut request_recv) = mpsc::channel(16);
        let (_, resp_recv) = mpsc::channel(16);

        let mut sink = CoordinatedRemoteSinkWriter::for_test(resp_recv, request_sender);
        let chunk = StreamChunk::from_pretty(
            " i T
            + 1 Ripper
        ",
        );

        // test epoch check
        assert!(
            tokio::time::timeout(Duration::from_secs(10), sink.barrier(true))
                .await
                .expect("test failed: should not commit without epoch")
                .is_err(),
            "test failed: no epoch check for commit()"
        );
        assert!(
            request_recv.try_recv().is_err(),
            "test failed: unchecked epoch before request"
        );

        assert!(
            tokio::time::timeout(Duration::from_secs(1), sink.write_batch(chunk))
                .await
                .expect("test failed: should not write without epoch")
                .is_err(),
            "test failed: no epoch check for write_batch()"
        );
        assert!(
            request_recv.try_recv().is_err(),
            "test failed: unchecked epoch before request"
        );
    }

    #[tokio::test]
    async fn test_remote_sink() {
        let (request_sender, mut request_receiver) = mpsc::channel(16);
        let (response_sender, response_receiver) = mpsc::channel(16);
        let mut sink = CoordinatedRemoteSinkWriter::for_test(response_receiver, request_sender);

        let chunk_a = StreamChunk::from_pretty(
            " i T
            + 1 Alice
            + 2 Bob
            + 3 Clare
        ",
        );
        let chunk_b = StreamChunk::from_pretty(
            " i T
            + 4 David
            + 5 Eve
            + 6 Frank
        ",
        );

        // test write batch
        sink.begin_epoch(2022).await.unwrap();
        assert_eq!(sink.epoch, Some(2022));

        sink.write_batch(chunk_a.clone()).await.unwrap();
        assert_eq!(sink.epoch, Some(2022));
        assert_eq!(sink.batch_id, 1);
        match request_receiver.recv().await.unwrap() {
            JniSinkWriterStreamRequest::Chunk {
                epoch,
                batch_id,
                chunk,
            } => {
                assert_eq!(epoch, 2022);
                assert_eq!(batch_id, 0);
                assert_eq!(chunk, chunk_a);
            }
            _ => panic!("test failed: failed to construct write request"),
        }

        // test commit
        response_sender
            .send(Ok(SinkWriterStreamResponse {
                response: Some(Response::Commit(CommitResponse {
                    epoch: 2022,
                    metadata: None,
                })),
            }))
            .await
            .expect("test failed: failed to sync epoch");
        sink.barrier(false).await.unwrap();
        let commit_request = request_receiver.recv().await.unwrap();
        match commit_request {
            JniSinkWriterStreamRequest::PbRequest(SinkWriterStreamRequest {
                request:
                    Some(Request::Barrier(Barrier {
                        epoch,
                        is_checkpoint: false,
                    })),
            }) => {
                assert_eq!(epoch, 2022);
            }
            _ => panic!("test failed: failed to construct sync request "),
        };

        // begin another epoch
        sink.begin_epoch(2023).await.unwrap();
        assert_eq!(sink.epoch, Some(2023));

        // test another write
        sink.write_batch(chunk_b.clone()).await.unwrap();
        assert_eq!(sink.epoch, Some(2023));
        assert_eq!(sink.batch_id, 2);
        match request_receiver.recv().await.unwrap() {
            JniSinkWriterStreamRequest::Chunk {
                epoch,
                batch_id,
                chunk,
            } => {
                assert_eq!(epoch, 2023);
                assert_eq!(batch_id, 1);
                assert_eq!(chunk, chunk_b);
            }
            _ => panic!("test failed: failed to construct write request"),
        }
    }
}
