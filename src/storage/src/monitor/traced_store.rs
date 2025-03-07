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

use std::ops::Bound;

use bytes::Bytes;
use futures::{Future, TryFutureExt, TryStreamExt};
use futures_async_stream::try_stream;
use risingwave_hummock_sdk::key::{TableKey, TableKeyRange};
use risingwave_hummock_sdk::HummockReadEpoch;
use risingwave_hummock_trace::{
    init_collector, should_use_trace, ConcurrentId, MayTraceSpan, OperationResult, StorageType,
    TraceResult, TraceSpan, TracedBytes, TracedSealCurrentEpochOptions, LOCAL_ID,
};
use thiserror_ext::AsReport;

use super::identity;
use crate::error::{StorageError, StorageResult};
use crate::hummock::sstable_store::SstableStoreRef;
use crate::hummock::{HummockStorage, SstableObjectIdManagerRef};
use crate::store::*;
use crate::StateStore;

#[derive(Clone)]
pub struct TracedStateStore<S> {
    inner: S,
    storage_type: StorageType,
}

impl<S> TracedStateStore<S> {
    pub fn new(inner: S, storage_type: StorageType) -> Self {
        if should_use_trace() {
            init_collector();
            tracing::info!("Hummock Tracing Enabled");
        }
        Self {
            inner,
            storage_type,
        }
    }

    pub fn new_global(inner: S) -> Self {
        Self::new(inner, StorageType::Global)
    }

    pub fn new_local(inner: S, options: NewLocalOptions) -> Self {
        let id = get_concurrent_id();
        let local_storage_id = rand::random::<u64>();
        let storage_type: StorageType = StorageType::Local(id, local_storage_id);
        let _span: MayTraceSpan =
            TraceSpan::new_local_storage_span(options.into(), storage_type, local_storage_id);

        Self {
            inner,
            storage_type,
        }
    }

    async fn traced_iter<'a, St: StateStoreIterItemStream>(
        &'a self,
        iter_stream_future: impl Future<Output = StorageResult<St>> + 'a,
        span: MayTraceSpan,
    ) -> StorageResult<TracedStateStoreIterStream<St>> {
        let res = iter_stream_future.await;
        if res.is_ok() {
            span.may_send_result(OperationResult::Iter(TraceResult::Ok(())));
        } else {
            span.may_send_result(OperationResult::Iter(TraceResult::Err));
        }
        let traced = TracedStateStoreIter::new(res?, span);
        Ok(traced.into_stream())
    }

    async fn traced_get(
        &self,
        key: TableKey<Bytes>,
        epoch: Option<u64>,
        read_options: ReadOptions,
        get_future: impl Future<Output = StorageResult<Option<Bytes>>>,
    ) -> StorageResult<Option<Bytes>> {
        let span = TraceSpan::new_get_span(
            key.0.clone(),
            epoch,
            read_options.clone().into(),
            self.storage_type,
        );

        let res = get_future.await;

        span.may_send_result(OperationResult::Get(TraceResult::from(
            res.as_ref()
                .map(|o| o.as_ref().map(|b| TracedBytes::from(b.clone()))),
        )));
        res
    }
}

type TracedStateStoreIterStream<S: StateStoreIterItemStream> = impl StateStoreIterItemStream;

impl<S: LocalStateStore> LocalStateStore for TracedStateStore<S> {
    type IterStream<'a> = impl StateStoreIterItemStream + 'a;

    fn may_exist(
        &self,
        key_range: TableKeyRange,
        read_options: ReadOptions,
    ) -> impl Future<Output = StorageResult<bool>> + Send + '_ {
        self.inner.may_exist(key_range, read_options)
    }

    fn get(
        &self,
        key: TableKey<Bytes>,
        read_options: ReadOptions,
    ) -> impl Future<Output = StorageResult<Option<Bytes>>> + '_ {
        self.traced_get(
            key.clone(),
            None,
            read_options.clone(),
            self.inner.get(key, read_options),
        )
    }

    fn iter(
        &self,
        key_range: TableKeyRange,
        read_options: ReadOptions,
    ) -> impl Future<Output = StorageResult<Self::IterStream<'_>>> + Send + '_ {
        let (l, r) = key_range.clone();
        let bytes_key_range = (l.map(|l| l.0), r.map(|r| r.0));
        let span = TraceSpan::new_iter_span(
            bytes_key_range,
            None,
            read_options.clone().into(),
            self.storage_type,
        );
        self.traced_iter(self.inner.iter(key_range, read_options), span)
            .map_ok(identity)
    }

    fn insert(
        &mut self,
        key: TableKey<Bytes>,
        new_val: Bytes,
        old_val: Option<Bytes>,
    ) -> StorageResult<()> {
        let span = TraceSpan::new_insert_span(
            key.0.clone(),
            new_val.clone(),
            old_val.clone(),
            self.storage_type,
        );
        let res = self.inner.insert(key, new_val, old_val);

        span.may_send_result(OperationResult::Insert(res.as_ref().map(|o| *o).into()));
        res
    }

    fn delete(&mut self, key: TableKey<Bytes>, old_val: Bytes) -> StorageResult<()> {
        let span = TraceSpan::new_delete_span(key.0.clone(), old_val.clone(), self.storage_type);

        let res = self.inner.delete(key, old_val);

        span.may_send_result(OperationResult::Delete(res.as_ref().map(|o| *o).into()));

        res
    }

    async fn flush(
        &mut self,
        delete_ranges: Vec<(Bound<Bytes>, Bound<Bytes>)>,
    ) -> StorageResult<usize> {
        let span = TraceSpan::new_flush_span(delete_ranges.clone(), self.storage_type);
        let res = self.inner.flush(delete_ranges).await;
        span.may_send_result(OperationResult::Flush(
            res.as_ref().map(|o: &usize| *o).into(),
        ));
        res
    }

    fn epoch(&self) -> u64 {
        let span = TraceSpan::new_epoch_span(self.storage_type);
        let res = self.inner.epoch();
        span.may_send_result(OperationResult::LocalStorageEpoch(TraceResult::Ok(res)));
        res
    }

    fn is_dirty(&self) -> bool {
        let span = TraceSpan::new_is_dirty_span(self.storage_type);
        let res = self.inner.is_dirty();
        span.may_send_result(OperationResult::LocalStorageIsDirty(TraceResult::Ok(res)));
        res
    }

    async fn init(&mut self, options: InitOptions) -> StorageResult<()> {
        let _span =
            TraceSpan::new_local_storage_init_span(options.clone().into(), self.storage_type);
        self.inner.init(options).await
    }

    fn seal_current_epoch(&mut self, next_epoch: u64, opts: SealCurrentEpochOptions) {
        let _span = TraceSpan::new_seal_current_epoch_span(
            next_epoch,
            TracedSealCurrentEpochOptions::from(opts.clone()),
            self.storage_type,
        );
        self.inner.seal_current_epoch(next_epoch, opts)
    }

    async fn try_flush(&mut self) -> StorageResult<()> {
        let span = TraceSpan::new_try_flush_span(self.storage_type);
        let res = self.inner.try_flush().await;
        span.may_send_result(OperationResult::TryFlush(res.as_ref().map(|o| *o).into()));
        res
    }
}

impl<S: StateStore> StateStore for TracedStateStore<S> {
    type Local = TracedStateStore<S::Local>;

    async fn try_wait_epoch(&self, epoch: HummockReadEpoch) -> StorageResult<()> {
        let span = TraceSpan::new_try_wait_epoch_span(epoch);

        let res = self.inner.try_wait_epoch(epoch).await;
        span.may_send_result(OperationResult::TryWaitEpoch(
            res.as_ref().map(|o| *o).into(),
        ));
        res
    }

    async fn sync(&self, epoch: u64) -> StorageResult<SyncResult> {
        let span: MayTraceSpan = TraceSpan::new_sync_span(epoch, self.storage_type);

        let sync_result = self.inner.sync(epoch).await;

        span.may_send_result(OperationResult::Sync(
            sync_result.as_ref().map(|res| res.sync_size).into(),
        ));
        sync_result
    }

    fn seal_epoch(&self, epoch: u64, is_checkpoint: bool) {
        let _span = TraceSpan::new_seal_span(epoch, is_checkpoint, self.storage_type);
        self.inner.seal_epoch(epoch, is_checkpoint);
    }

    async fn clear_shared_buffer(&self) -> StorageResult<()> {
        let span = TraceSpan::new_clear_shared_buffer_span();
        let res = self.inner.clear_shared_buffer().await;
        span.may_send_result(OperationResult::ClearSharedBuffer(
            res.as_ref().map(|o| *o).into(),
        ));
        res
    }

    async fn new_local(&self, options: NewLocalOptions) -> Self::Local {
        TracedStateStore::new_local(self.inner.new_local(options.clone()).await, options)
    }

    fn validate_read_epoch(&self, epoch: HummockReadEpoch) -> StorageResult<()> {
        let span = TraceSpan::new_validate_read_epoch_span(epoch);
        let res = self.inner.validate_read_epoch(epoch);
        span.may_send_result(OperationResult::ValidateReadEpoch(
            res.as_ref().map(|o| *o).into(),
        ));
        res
    }
}

impl<S: StateStoreRead> StateStoreRead for TracedStateStore<S> {
    type IterStream = impl StateStoreReadIterStream;

    fn get(
        &self,
        key: TableKey<Bytes>,
        epoch: u64,
        read_options: ReadOptions,
    ) -> impl Future<Output = StorageResult<Option<Bytes>>> + Send + '_ {
        self.traced_get(
            key.clone(),
            Some(epoch),
            read_options.clone(),
            self.inner.get(key, epoch, read_options),
        )
    }

    fn iter(
        &self,
        key_range: TableKeyRange,
        epoch: u64,
        read_options: ReadOptions,
    ) -> impl Future<Output = StorageResult<Self::IterStream>> + '_ {
        let (l, r) = key_range.clone();
        let bytes_key_range = (l.map(|l| l.0), r.map(|r| r.0));
        let span = TraceSpan::new_iter_span(
            bytes_key_range,
            Some(epoch),
            read_options.clone().into(),
            self.storage_type,
        );
        self.traced_iter(self.inner.iter(key_range, epoch, read_options), span)
            .map_ok(identity)
    }
}

impl TracedStateStore<HummockStorage> {
    pub fn sstable_store(&self) -> SstableStoreRef {
        self.inner.sstable_store()
    }

    pub fn sstable_object_id_manager(&self) -> &SstableObjectIdManagerRef {
        self.inner.sstable_object_id_manager()
    }
}

impl<S> TracedStateStore<S> {
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S> Drop for TracedStateStore<S> {
    fn drop(&mut self) {
        if let StorageType::Local(_, _) = self.storage_type {
            let _ = TraceSpan::new_drop_storage_span(self.storage_type);
        }
    }
}

pub struct TracedStateStoreIter<S> {
    inner: S,
    span: MayTraceSpan,
}

impl<S> TracedStateStoreIter<S> {
    fn new(inner: S, span: MayTraceSpan) -> Self {
        TracedStateStoreIter { inner, span }
    }
}

impl<S: StateStoreIterItemStream> TracedStateStoreIter<S> {
    #[try_stream(ok = StateStoreIterItem, error = StorageError)]
    async fn into_stream_inner(self) {
        let inner = self.inner;
        futures::pin_mut!(inner);

        while let Some((key, value)) = inner
            .try_next()
            .await
            .inspect_err(|e| tracing::error!(error = %e.as_report(), "Failed in next"))?
        {
            self.span.may_send_iter_next();
            self.span
                .may_send_result(OperationResult::IterNext(TraceResult::Ok(Some((
                    TracedBytes::from(key.user_key.table_key.to_vec()),
                    TracedBytes::from(value.clone()),
                )))));
            yield (key, value);
        }
    }

    fn into_stream(self) -> TracedStateStoreIterStream<S> {
        Self::into_stream_inner(self)
    }
}

pub fn get_concurrent_id() -> ConcurrentId {
    LOCAL_ID.get()
}
