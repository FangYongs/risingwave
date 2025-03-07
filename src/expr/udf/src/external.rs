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

use std::time::Duration;

use arrow_array::RecordBatch;
use arrow_flight::decode::FlightRecordBatchStream;
use arrow_flight::encode::FlightDataEncoderBuilder;
use arrow_flight::error::FlightError;
use arrow_flight::flight_service_client::FlightServiceClient;
use arrow_flight::{FlightData, FlightDescriptor};
use arrow_schema::Schema;
use cfg_or_panic::cfg_or_panic;
use futures_util::{stream, Stream, StreamExt, TryStreamExt};
use thiserror_ext::AsReport;
use tonic::transport::Channel;

use crate::metrics::GLOBAL_METRICS;
use crate::{Error, Result};

/// Client for external function service based on Arrow Flight.
#[derive(Debug)]
pub struct ArrowFlightUdfClient {
    client: FlightServiceClient<Channel>,
    addr: String,
}

// TODO: support UDF in simulation
#[cfg_or_panic(not(madsim))]
impl ArrowFlightUdfClient {
    /// Connect to a UDF service.
    pub async fn connect(addr: &str) -> Result<Self> {
        let conn = tonic::transport::Endpoint::new(addr.to_string())?
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .connect()
            .await?;
        let client = FlightServiceClient::new(conn);
        Ok(Self {
            client,
            addr: addr.into(),
        })
    }

    /// Connect to a UDF service lazily (i.e. only when the first request is sent).
    pub fn connect_lazy(addr: &str) -> Result<Self> {
        let conn = tonic::transport::Endpoint::new(addr.to_string())?
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .connect_lazy();
        let client = FlightServiceClient::new(conn);
        Ok(Self {
            client,
            addr: addr.into(),
        })
    }

    /// Check if the function is available and the schema is match.
    pub async fn check(&self, id: &str, args: &Schema, returns: &Schema) -> Result<()> {
        let descriptor = FlightDescriptor::new_path(vec![id.into()]);

        let response = self.client.clone().get_flight_info(descriptor).await?;

        // check schema
        let info = response.into_inner();
        let input_num = info.total_records as usize;
        let full_schema = Schema::try_from(info).map_err(|e| {
            FlightError::DecodeError(format!("Error decoding schema: {}", e.as_report()))
        })?;
        if input_num > full_schema.fields.len() {
            return Err(Error::service_error(format!(
                "function {:?} schema info not consistency: input_num: {}, total_fields: {}",
                id,
                input_num,
                full_schema.fields.len()
            )));
        }

        let (input_fields, return_fields) = full_schema.fields.split_at(input_num);
        let actual_input_types: Vec<_> = input_fields.iter().map(|f| f.data_type()).collect();
        let actual_result_types: Vec<_> = return_fields.iter().map(|f| f.data_type()).collect();
        let expect_input_types: Vec<_> = args.fields.iter().map(|f| f.data_type()).collect();
        let expect_result_types: Vec<_> = returns.fields.iter().map(|f| f.data_type()).collect();
        if !data_types_match(&expect_input_types, &actual_input_types) {
            return Err(Error::type_mismatch(format!(
                "function: {:?}, expect arguments: {:?}, actual: {:?}",
                id, expect_input_types, actual_input_types
            )));
        }
        if !data_types_match(&expect_result_types, &actual_result_types) {
            return Err(Error::type_mismatch(format!(
                "function: {:?}, expect return: {:?}, actual: {:?}",
                id, expect_result_types, actual_result_types
            )));
        }
        Ok(())
    }

    /// Call a function.
    pub async fn call(&self, id: &str, input: RecordBatch) -> Result<RecordBatch> {
        let metrics = &*GLOBAL_METRICS;
        let labels = &[self.addr.as_str(), id];
        metrics
            .udf_input_chunk_rows
            .with_label_values(labels)
            .observe(input.num_rows() as f64);
        metrics
            .udf_input_rows
            .with_label_values(labels)
            .inc_by(input.num_rows() as u64);
        metrics
            .udf_input_bytes
            .with_label_values(labels)
            .inc_by(input.get_array_memory_size() as u64);
        let timer = metrics.udf_latency.with_label_values(labels).start_timer();

        let result = self.call_internal(id, input).await;

        timer.stop_and_record();
        if result.is_ok() {
            &metrics.udf_success_count
        } else {
            &metrics.udf_failure_count
        }
        .with_label_values(labels)
        .inc();
        result
    }

    async fn call_internal(&self, id: &str, input: RecordBatch) -> Result<RecordBatch> {
        let mut output_stream = self.call_stream(id, stream::once(async { input })).await?;
        // TODO: support no output
        let head = output_stream
            .next()
            .await
            .ok_or_else(Error::no_returned)??;
        let remaining = output_stream.try_collect::<Vec<_>>().await?;
        if remaining.is_empty() {
            Ok(head)
        } else {
            Ok(arrow_select::concat::concat_batches(
                &head.schema(),
                std::iter::once(&head).chain(remaining.iter()),
            )?)
        }
    }

    /// Call a function, retry up to 5 times / 3s if connection is broken.
    pub async fn call_with_retry(&self, id: &str, input: RecordBatch) -> Result<RecordBatch> {
        let mut backoff = Duration::from_millis(100);
        for i in 0..5 {
            match self.call(id, input.clone()).await {
                Err(err) if err.is_connection_error() && i != 4 => {
                    tracing::error!(error = %err.as_report(), "UDF connection error. retry...");
                }
                ret => return ret,
            }
            tokio::time::sleep(backoff).await;
            backoff *= 2;
        }
        unreachable!()
    }

    /// Call a function with streaming input and output.
    #[panic_return = "Result<stream::Empty<_>>"]
    pub async fn call_stream(
        &self,
        id: &str,
        inputs: impl Stream<Item = RecordBatch> + Send + 'static,
    ) -> Result<impl Stream<Item = Result<RecordBatch>> + Send + 'static> {
        let descriptor = FlightDescriptor::new_path(vec![id.into()]);
        let flight_data_stream =
            FlightDataEncoderBuilder::new()
                .build(inputs.map(Ok))
                .map(move |res| FlightData {
                    // TODO: fill descriptor only for the first message
                    flight_descriptor: Some(descriptor.clone()),
                    ..res.unwrap()
                });

        // call `do_exchange` on Flight server
        let response = self.client.clone().do_exchange(flight_data_stream).await?;

        // decode response
        let stream = response.into_inner();
        let record_batch_stream = FlightRecordBatchStream::new_from_flight_data(
            // convert tonic::Status to FlightError
            stream.map_err(|e| e.into()),
        );
        Ok(record_batch_stream.map_err(|e| e.into()))
    }
}

/// Check if two list of data types match, ignoring field names.
fn data_types_match(a: &[&arrow_schema::DataType], b: &[&arrow_schema::DataType]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    #[allow(clippy::disallowed_methods)]
    a.iter().zip(b.iter()).all(|(a, b)| a.equals_datatype(b))
}
