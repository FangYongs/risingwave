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

use std::collections::HashMap;
use std::default::Default;
use std::ops::Bound;
use std::ops::Bound::*;
use std::sync::Arc;

use bytes::{BufMut, Bytes, BytesMut};
use either::Either;
use futures::{pin_mut, FutureExt, Stream, StreamExt};
use futures_async_stream::for_await;
use itertools::{izip, Itertools};
use risingwave_common::array::stream_record::Record;
use risingwave_common::array::{ArrayImplBuilder, ArrayRef, DataChunk, Op, StreamChunk};
use risingwave_common::buffer::Bitmap;
use risingwave_common::cache::CachePriority;
use risingwave_common::catalog::{
    get_dist_key_in_pk_indices, ColumnDesc, ColumnId, TableId, TableOption,
};
use risingwave_common::hash::{VirtualNode, VnodeBitmapExt};
use risingwave_common::row::{self, once, CompactedRow, Once, OwnedRow, Row, RowExt};
use risingwave_common::types::{DataType, Datum, DefaultOrd, DefaultOrdered, ScalarImpl};
use risingwave_common::util::column_index_mapping::ColIndexMapping;
use risingwave_common::util::epoch::EpochPair;
use risingwave_common::util::iter_util::{ZipEqDebug, ZipEqFast};
use risingwave_common::util::row_serde::OrderedRowSerde;
use risingwave_common::util::sort_util::OrderType;
use risingwave_common::util::value_encoding::BasicSerde;
use risingwave_hummock_sdk::key::{
    end_bound_of_prefix, prefixed_range_with_vnode, range_of_prefix,
    start_bound_of_excluded_prefix, TableKey, TableKeyRange,
};
use risingwave_hummock_sdk::table_watermark::{VnodeWatermark, WatermarkDirection};
use risingwave_pb::catalog::Table;
use risingwave_storage::error::{ErrorKind, StorageError, StorageResult};
use risingwave_storage::hummock::CachePolicy;
use risingwave_storage::mem_table::MemTableError;
use risingwave_storage::row_serde::find_columns_by_ids;
use risingwave_storage::row_serde::row_serde_util::{
    deserialize_pk_with_vnode, serialize_pk, serialize_pk_with_vnode,
};
use risingwave_storage::row_serde::value_serde::ValueRowSerde;
use risingwave_storage::store::{
    InitOptions, LocalStateStore, NewLocalOptions, OpConsistencyLevel, PrefetchOptions,
    ReadOptions, SealCurrentEpochOptions, StateStoreIterItemStream,
};
use risingwave_storage::table::merge_sort::merge_sort;
use risingwave_storage::table::{KeyedRow, TableDistribution};
use risingwave_storage::StateStore;
use thiserror_ext::AsReport;
use tracing::{trace, Instrument};

use super::watermark::{WatermarkBufferByEpoch, WatermarkBufferStrategy};
use crate::cache::cache_may_stale;
use crate::common::cache::{StateCache, StateCacheFiller};
use crate::common::table::state_table_cache::StateTableWatermarkCache;
use crate::executor::{StreamExecutorError, StreamExecutorResult};

/// This num is arbitrary and we may want to improve this choice in the future.
const STATE_CLEANING_PERIOD_EPOCH: usize = 300;
/// Mostly watermark operators will have inserts (append-only).
/// So this number should not need to be very large.
/// But we may want to improve this choice in the future.
const WATERMARK_CACHE_ENTRIES: usize = 16;

type DefaultWatermarkBufferStrategy = WatermarkBufferByEpoch<STATE_CLEANING_PERIOD_EPOCH>;

/// `StateTableInner` is the interface accessing relational data in KV(`StateStore`) with
/// row-based encoding.
#[derive(Clone)]
pub struct StateTableInner<
    S,
    SD = BasicSerde,
    const IS_REPLICATED: bool = false,
    W = DefaultWatermarkBufferStrategy,
    const USE_WATERMARK_CACHE: bool = false,
> where
    S: StateStore,
    SD: ValueRowSerde,
    W: WatermarkBufferStrategy,
{
    /// Id for this table.
    table_id: TableId,

    /// State store backend.
    local_store: S::Local,

    /// Used for serializing and deserializing the primary key.
    pk_serde: OrderedRowSerde,

    /// Row deserializer with value encoding
    row_serde: SD,

    /// Indices of primary key.
    /// Note that the index is based on the all columns of the table, instead of the output ones.
    // FIXME: revisit constructions and usages.
    pk_indices: Vec<usize>,

    /// Distribution of the state table.
    ///
    /// It holds vnode bitmap. Only the rows whose vnode of the primary key is in this set will be visible to the
    /// executor. The table will also check whether the written rows
    /// conform to this partition.
    distribution: TableDistribution,

    prefix_hint_len: usize,

    /// Used for catalog table_properties
    table_option: TableOption,

    value_indices: Option<Vec<usize>>,

    /// Strategy to buffer watermark for lazy state cleaning.
    watermark_buffer_strategy: W,
    /// State cleaning watermark. Old states will be cleaned under this watermark when committing.
    state_clean_watermark: Option<ScalarImpl>,

    /// Watermark of the last committed state cleaning.
    prev_cleaned_watermark: Option<ScalarImpl>,

    /// Watermark cache
    watermark_cache: StateTableWatermarkCache,

    /// Data Types
    /// We will need to use to build data chunks from state table rows.
    data_types: Vec<DataType>,

    /// "i" here refers to the base state_table's actual schema.
    /// "o" here refers to the replicated state table's output schema.
    /// This mapping is used to reconstruct a row being written from replicated state table.
    /// Such that the schema of this row will match the full schema of the base state table.
    /// It is only applicable for replication.
    i2o_mapping: ColIndexMapping,

    /// Output indices
    /// Used for:
    /// 1. Computing output_value_indices to ser/de replicated rows.
    /// 2. Computing output pk indices to used them for backfill state.
    output_indices: Vec<usize>,

    is_consistent_op: bool,
}

/// `StateTable` will use `BasicSerde` as default
pub type StateTable<S> = StateTableInner<S, BasicSerde>;
/// `ReplicatedStateTable` is meant to replicate upstream shared buffer.
/// Used for `ArrangementBackfill` executor.
pub type ReplicatedStateTable<S, SD> = StateTableInner<S, SD, true>;
/// `WatermarkCacheStateTable` caches the watermark column.
/// It will reduce state cleaning overhead.
pub type WatermarkCacheStateTable<S> =
    StateTableInner<S, BasicSerde, false, DefaultWatermarkBufferStrategy, true>;
pub type WatermarkCacheParameterizedStateTable<S, const USE_WATERMARK_CACHE: bool> =
    StateTableInner<S, BasicSerde, false, DefaultWatermarkBufferStrategy, USE_WATERMARK_CACHE>;

// initialize
impl<S, SD, W, const USE_WATERMARK_CACHE: bool> StateTableInner<S, SD, true, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
    W: WatermarkBufferStrategy,
{
    /// get the newest epoch of the state store and panic if the `init_epoch()` has never be called
    /// async interface only used for replicated state table,
    /// as it needs to wait for prev epoch to be committed.
    pub async fn init_epoch(&mut self, epoch: EpochPair) -> StorageResult<()> {
        self.local_store
            .init(InitOptions::new_with_epoch(epoch))
            .await
    }
}

// initialize
impl<S, SD, W, const USE_WATERMARK_CACHE: bool>
    StateTableInner<S, SD, false, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
    W: WatermarkBufferStrategy,
{
    /// get the newest epoch of the state store and panic if the `init_epoch()` has never be called
    /// No need to `wait_for_epoch`, so it should complete immediately.
    pub fn init_epoch(&mut self, epoch: EpochPair) {
        self.local_store
            .init(InitOptions::new_with_epoch(epoch))
            .now_or_never()
            .expect("non-replicated state store should start immediately.")
            .expect("non-replicated state store should not wait_for_epoch, and fail because of it.")
    }
}

fn consistent_old_value_op(row_serde: impl ValueRowSerde) -> OpConsistencyLevel {
    OpConsistencyLevel::ConsistentOldValue(Arc::new(move |first: &Bytes, second: &Bytes| {
        if first == second {
            return true;
        }
        let first = match row_serde.deserialize(first) {
            Ok(rows) => rows,
            Err(e) => {
                error!(error = %e.as_report(), value = ?first, "fail to deserialize serialized value");
                return false;
            }
        };
        let second = match row_serde.deserialize(second) {
            Ok(rows) => rows,
            Err(e) => {
                error!(error = %e.as_report(), value = ?second, "fail to deserialize serialized value");
                return false;
            }
        };
        if first != second {
            error!(first = ?first, second = ?second, "sanity check fail");
            false
        } else {
            true
        }
    }))
}

// initialize
// FIXME(kwannoel): Enforce that none of the constructors here
// should be used by replicated state table.
// Apart from from_table_catalog_inner.
impl<S, SD, const IS_REPLICATED: bool, W, const USE_WATERMARK_CACHE: bool>
    StateTableInner<S, SD, IS_REPLICATED, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
    W: WatermarkBufferStrategy,
{
    /// Create state table from table catalog and store.
    pub async fn from_table_catalog(
        table_catalog: &Table,
        store: S,
        vnodes: Option<Arc<Bitmap>>,
    ) -> Self {
        Self::from_table_catalog_inner(table_catalog, store, vnodes, true, vec![]).await
    }

    /// Create state table from table catalog and store with sanity check disabled.
    pub async fn from_table_catalog_inconsistent_op(
        table_catalog: &Table,
        store: S,
        vnodes: Option<Arc<Bitmap>>,
    ) -> Self {
        Self::from_table_catalog_inner(table_catalog, store, vnodes, false, vec![]).await
    }

    /// Create state table from table catalog and store.
    async fn from_table_catalog_inner(
        table_catalog: &Table,
        store: S,
        vnodes: Option<Arc<Bitmap>>,
        is_consistent_op: bool,
        output_column_ids: Vec<ColumnId>,
    ) -> Self {
        let table_id = TableId::new(table_catalog.id);
        let table_columns: Vec<ColumnDesc> = table_catalog
            .columns
            .iter()
            .map(|col| col.column_desc.as_ref().unwrap().into())
            .collect();
        let data_types: Vec<DataType> = table_catalog
            .columns
            .iter()
            .map(|col| {
                col.get_column_desc()
                    .unwrap()
                    .get_column_type()
                    .unwrap()
                    .into()
            })
            .collect();
        let order_types: Vec<OrderType> = table_catalog
            .pk
            .iter()
            .map(|col_order| OrderType::from_protobuf(col_order.get_order_type().unwrap()))
            .collect();
        let dist_key_indices: Vec<usize> = table_catalog
            .distribution_key
            .iter()
            .map(|dist_index| *dist_index as usize)
            .collect();

        let pk_indices = table_catalog
            .pk
            .iter()
            .map(|col_order| col_order.column_index as usize)
            .collect_vec();

        // FIXME(yuhao): only use `dist_key_in_pk` in the proto
        let dist_key_in_pk_indices = if table_catalog.get_dist_key_in_pk().is_empty() {
            get_dist_key_in_pk_indices(&dist_key_indices, &pk_indices)
        } else {
            table_catalog
                .get_dist_key_in_pk()
                .iter()
                .map(|idx| *idx as usize)
                .collect()
        };

        let vnode_col_idx_in_pk = table_catalog.vnode_col_index.as_ref().and_then(|idx| {
            let vnode_col_idx = *idx as usize;
            pk_indices.iter().position(|&i| vnode_col_idx == i)
        });

        let distribution =
            TableDistribution::new(vnodes, dist_key_in_pk_indices, vnode_col_idx_in_pk);

        let pk_data_types = pk_indices
            .iter()
            .map(|i| table_columns[*i].data_type.clone())
            .collect();
        let pk_serde = OrderedRowSerde::new(pk_data_types, order_types);

        let input_value_indices = table_catalog
            .value_indices
            .iter()
            .map(|val| *val as usize)
            .collect_vec();

        let no_shuffle_value_indices = (0..table_columns.len()).collect_vec();

        // if value_indices is the no shuffle full columns.
        let value_indices = match input_value_indices.len() == table_columns.len()
            && input_value_indices == no_shuffle_value_indices
        {
            true => None,
            false => Some(input_value_indices),
        };
        let prefix_hint_len = table_catalog.read_prefix_len_hint as usize;

        let make_row_serde = || {
            SD::new(
                Arc::from_iter(table_catalog.value_indices.iter().map(|val| *val as usize)),
                Arc::from(table_columns.clone().into_boxed_slice()),
            )
        };

        let op_consistency_level = if is_consistent_op {
            let row_serde = make_row_serde();
            consistent_old_value_op(row_serde)
        } else {
            OpConsistencyLevel::Inconsistent
        };

        let table_option = TableOption::build_table_option(table_catalog.get_properties());
        let new_local_options = if IS_REPLICATED {
            NewLocalOptions::new_replicated(table_id, op_consistency_level, table_option)
        } else {
            NewLocalOptions::new(table_id, op_consistency_level, table_option)
        };
        let local_state_store = store.new_local(new_local_options).await;

        let row_serde = make_row_serde();

        // If state table has versioning, that means it supports
        // Schema change. In that case, the row encoding should be column aware as well.
        // Otherwise both will be false.
        // NOTE(kwannoel): Replicated table will follow upstream table's versioning. I'm not sure
        // If ALTER TABLE will propagate to this replicated table as well. Ideally it won't
        assert_eq!(
            table_catalog.version.is_some(),
            row_serde.kind().is_column_aware()
        );

        let watermark_cache = if USE_WATERMARK_CACHE {
            StateTableWatermarkCache::new(WATERMARK_CACHE_ENTRIES)
        } else {
            StateTableWatermarkCache::new(0)
        };

        // Get info for replicated state table.
        let output_column_ids_to_input_idx = output_column_ids
            .iter()
            .enumerate()
            .map(|(pos, id)| (*id, pos))
            .collect::<HashMap<_, _>>();

        // Compute column descriptions
        let columns: Vec<ColumnDesc> = table_catalog
            .columns
            .iter()
            .map(|c| c.column_desc.as_ref().unwrap().into())
            .collect_vec();

        // Compute i2o mapping
        let mut i2o_mapping = vec![None; columns.len()];
        let mut output_column_indices = vec![];
        for (i, column) in columns.iter().enumerate() {
            if let Some(pos) = output_column_ids_to_input_idx.get(&column.column_id) {
                i2o_mapping[i] = Some(*pos);
                output_column_indices.push(i);
            }
        }
        let i2o_mapping = ColIndexMapping::new(i2o_mapping, output_column_indices.len());

        // Compute output indices
        let (_, output_indices) = find_columns_by_ids(&columns[..], &output_column_ids);

        Self {
            table_id,
            local_store: local_state_store,
            pk_serde,
            row_serde,
            pk_indices,
            distribution,
            prefix_hint_len,
            table_option,
            value_indices,
            watermark_buffer_strategy: W::default(),
            state_clean_watermark: None,
            prev_cleaned_watermark: None,
            watermark_cache,
            data_types,
            output_indices,
            i2o_mapping,
            is_consistent_op,
        }
    }

    /// Create a state table without distribution, used for unit tests.
    pub async fn new_without_distribution(
        store: S,
        table_id: TableId,
        columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
    ) -> Self {
        Self::new_with_distribution(
            store,
            table_id,
            columns,
            order_types,
            pk_indices,
            TableDistribution::singleton(),
            None,
        )
        .await
    }

    /// Create a state table without distribution, with given `value_indices`, used for unit tests.
    pub async fn new_without_distribution_with_value_indices(
        store: S,
        table_id: TableId,
        columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
        value_indices: Vec<usize>,
    ) -> Self {
        Self::new_with_distribution(
            store,
            table_id,
            columns,
            order_types,
            pk_indices,
            TableDistribution::singleton(),
            Some(value_indices),
        )
        .await
    }

    /// Create a state table without distribution, used for unit tests.
    pub async fn new_without_distribution_inconsistent_op(
        store: S,
        table_id: TableId,
        columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
    ) -> Self {
        Self::new_with_distribution_inner(
            store,
            table_id,
            columns,
            order_types,
            pk_indices,
            TableDistribution::singleton(),
            None,
            false,
        )
        .await
    }

    /// Create a state table with distribution specified with `distribution`. Should use
    /// `Distribution::fallback()` for tests.
    pub async fn new_with_distribution(
        store: S,
        table_id: TableId,
        table_columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
        distribution: TableDistribution,
        value_indices: Option<Vec<usize>>,
    ) -> Self {
        Self::new_with_distribution_inner(
            store,
            table_id,
            table_columns,
            order_types,
            pk_indices,
            distribution,
            value_indices,
            true,
        )
        .await
    }

    pub async fn new_with_distribution_inconsistent_op(
        store: S,
        table_id: TableId,
        table_columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
        distribution: TableDistribution,
        value_indices: Option<Vec<usize>>,
    ) -> Self {
        Self::new_with_distribution_inner(
            store,
            table_id,
            table_columns,
            order_types,
            pk_indices,
            distribution,
            value_indices,
            false,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn new_with_distribution_inner(
        store: S,
        table_id: TableId,
        table_columns: Vec<ColumnDesc>,
        order_types: Vec<OrderType>,
        pk_indices: Vec<usize>,
        distribution: TableDistribution,
        value_indices: Option<Vec<usize>>,
        is_consistent_op: bool,
    ) -> Self {
        let make_row_serde = || {
            SD::new(
                Arc::from(
                    value_indices
                        .clone()
                        .unwrap_or_else(|| (0..table_columns.len()).collect_vec())
                        .into_boxed_slice(),
                ),
                Arc::from(table_columns.clone().into_boxed_slice()),
            )
        };
        let op_consistency_level = if is_consistent_op {
            let row_serde = make_row_serde();
            consistent_old_value_op(row_serde)
        } else {
            OpConsistencyLevel::Inconsistent
        };
        let local_state_store = store
            .new_local(NewLocalOptions::new(
                table_id,
                op_consistency_level,
                TableOption::default(),
            ))
            .await;
        let row_serde = make_row_serde();
        let data_types: Vec<DataType> = table_columns
            .iter()
            .map(|col| col.data_type.clone())
            .collect();
        let pk_data_types = pk_indices
            .iter()
            .map(|i| table_columns[*i].data_type.clone())
            .collect();
        let pk_serde = OrderedRowSerde::new(pk_data_types, order_types);

        let watermark_cache = if USE_WATERMARK_CACHE {
            StateTableWatermarkCache::new(WATERMARK_CACHE_ENTRIES)
        } else {
            StateTableWatermarkCache::new(0)
        };
        Self {
            table_id,
            local_store: local_state_store,
            pk_serde,
            row_serde,
            pk_indices,
            distribution,
            prefix_hint_len: 0,
            table_option: Default::default(),
            value_indices,
            watermark_buffer_strategy: W::default(),
            state_clean_watermark: None,
            prev_cleaned_watermark: None,
            watermark_cache,
            data_types,
            output_indices: vec![],
            i2o_mapping: ColIndexMapping::new(vec![], 0),
            is_consistent_op,
        }
    }

    pub fn get_data_types(&self) -> &[DataType] {
        &self.data_types
    }

    pub fn table_id(&self) -> u32 {
        self.table_id.table_id
    }

    /// get the newest epoch of the state store and panic if the `init_epoch()` has never be called
    pub fn epoch(&self) -> u64 {
        self.local_store.epoch()
    }

    /// Get the vnode value with given (prefix of) primary key
    fn compute_prefix_vnode(&self, pk_prefix: impl Row) -> VirtualNode {
        self.distribution
            .try_compute_vnode_by_pk_prefix(pk_prefix)
            .expect("For streaming, the given prefix must be enough to calculate the vnode")
    }

    /// Get the vnode value of the given primary key
    pub fn compute_vnode_by_pk(&self, pk: impl Row) -> VirtualNode {
        self.distribution.compute_vnode_by_pk(pk)
    }

    /// NOTE(kwannoel): This is used by backfill.
    /// We want to check pk indices of upstream table.
    pub fn pk_indices(&self) -> &[usize] {
        &self.pk_indices
    }

    /// Get the indices of the primary key columns in the output columns.
    ///
    /// Returns `None` if any of the primary key columns is not in the output columns.
    pub fn pk_in_output_indices(&self) -> Option<Vec<usize>> {
        assert!(IS_REPLICATED);
        self.pk_indices
            .iter()
            .map(|&i| self.output_indices.iter().position(|&j| i == j))
            .collect()
    }

    pub fn pk_serde(&self) -> &OrderedRowSerde {
        &self.pk_serde
    }

    pub fn vnodes(&self) -> &Arc<Bitmap> {
        self.distribution.vnodes()
    }

    pub fn value_indices(&self) -> &Option<Vec<usize>> {
        &self.value_indices
    }

    fn is_dirty(&self) -> bool {
        self.local_store.is_dirty() || self.state_clean_watermark.is_some()
    }

    pub fn is_consistent_op(&self) -> bool {
        self.is_consistent_op
    }
}

impl<S, SD, W, const USE_WATERMARK_CACHE: bool> StateTableInner<S, SD, true, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
    W: WatermarkBufferStrategy,
{
    /// Create replicated state table from table catalog with output indices
    pub async fn from_table_catalog_with_output_column_ids(
        table_catalog: &Table,
        store: S,
        vnodes: Option<Arc<Bitmap>>,
        output_column_ids: Vec<ColumnId>,
    ) -> Self {
        Self::from_table_catalog_inner(table_catalog, store, vnodes, false, output_column_ids).await
    }
}

// point get
impl<
        S,
        SD,
        const IS_REPLICATED: bool,
        W: WatermarkBufferStrategy,
        const USE_WATERMARK_CACHE: bool,
    > StateTableInner<S, SD, IS_REPLICATED, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
{
    /// Get a single row from state table.
    pub async fn get_row(&self, pk: impl Row) -> StreamExecutorResult<Option<OwnedRow>> {
        let encoded_row: Option<Bytes> = self.get_encoded_row(pk).await?;
        match encoded_row {
            Some(encoded_row) => {
                let row = self.row_serde.deserialize(&encoded_row)?;
                if IS_REPLICATED {
                    // If the table is replicated, we need to deserialize the row with the output
                    // indices.
                    let row = row.project(&self.output_indices);
                    Ok(Some(row.into_owned_row()))
                } else {
                    Ok(Some(OwnedRow::new(row)))
                }
            }
            None => Ok(None),
        }
    }

    /// Get a raw encoded row from state table.
    pub async fn get_encoded_row(&self, pk: impl Row) -> StreamExecutorResult<Option<Bytes>> {
        assert!(pk.len() <= self.pk_indices.len());

        if self.prefix_hint_len != 0 {
            debug_assert_eq!(self.prefix_hint_len, pk.len());
        }

        let serialized_pk =
            serialize_pk_with_vnode(&pk, &self.pk_serde, self.compute_vnode_by_pk(&pk));

        let prefix_hint = if self.prefix_hint_len != 0 && self.prefix_hint_len == pk.len() {
            Some(serialized_pk.slice(VirtualNode::SIZE..))
        } else {
            None
        };

        let read_options = ReadOptions {
            prefix_hint,
            retention_seconds: self.table_option.retention_seconds,
            table_id: self.table_id,
            cache_policy: CachePolicy::Fill(CachePriority::High),
            ..Default::default()
        };

        self.local_store
            .get(serialized_pk, read_options)
            .await
            .map_err(Into::into)
    }

    /// Get a row in value-encoding format from state table.
    pub async fn get_compacted_row(
        &self,
        pk: impl Row,
    ) -> StreamExecutorResult<Option<CompactedRow>> {
        if self.row_serde.kind().is_basic() {
            // Basic serde is in value-encoding format, which is compatible with the compacted row.
            self.get_encoded_row(pk)
                .await
                .map(|bytes| bytes.map(CompactedRow::new))
        } else {
            // For other encodings, we must first deserialize it into a `Row` first, then serialize
            // it back into value-encoding format.
            self.get_row(pk)
                .await
                .map(|row| row.map(CompactedRow::from))
        }
    }

    /// Update the vnode bitmap of the state table, returns the previous vnode bitmap.
    #[must_use = "the executor should decide whether to manipulate the cache based on the previous vnode bitmap"]
    pub fn update_vnode_bitmap(&mut self, new_vnodes: Arc<Bitmap>) -> (Arc<Bitmap>, bool) {
        assert!(
            !self.is_dirty(),
            "vnode bitmap should only be updated when state table is clean"
        );
        if self.distribution.is_singleton() {
            assert_eq!(
                &new_vnodes,
                self.vnodes(),
                "should not update vnode bitmap for singleton table"
            );
        }
        assert_eq!(self.vnodes().len(), new_vnodes.len());

        let cache_may_stale = cache_may_stale(self.vnodes(), &new_vnodes);

        if cache_may_stale {
            self.state_clean_watermark = None;
            if USE_WATERMARK_CACHE {
                self.watermark_cache.clear();
            }
        }

        (
            self.distribution.update_vnode_bitmap(new_vnodes),
            cache_may_stale,
        )
    }
}

// write
impl<
        S,
        SD,
        const IS_REPLICATED: bool,
        W: WatermarkBufferStrategy,
        const USE_WATERMARK_CACHE: bool,
    > StateTableInner<S, SD, IS_REPLICATED, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
{
    fn handle_mem_table_error(&self, e: StorageError) {
        let e = match e.into_inner() {
            ErrorKind::MemTable(e) => e,
            _ => unreachable!("should only get memtable error"),
        };
        match *e {
            MemTableError::InconsistentOperation { key, prev, new } => {
                let (vnode, key) = deserialize_pk_with_vnode(&key, &self.pk_serde).unwrap();
                panic!(
                    "mem-table operation inconsistent! table_id: {}, vnode: {}, key: {:?}, prev: {}, new: {}",
                    self.table_id(),
                    vnode,
                    &key,
                    prev.debug_fmt(&self.row_serde),
                    new.debug_fmt(&self.row_serde),
                )
            }
        }
    }

    fn serialize_value(&self, value: impl Row) -> Bytes {
        if let Some(value_indices) = self.value_indices.as_ref() {
            self.row_serde
                .serialize(value.project(value_indices))
                .into()
        } else {
            self.row_serde.serialize(value).into()
        }
    }

    fn insert_inner(&mut self, key: TableKey<Bytes>, value_bytes: Bytes) {
        self.local_store
            .insert(key, value_bytes, None)
            .unwrap_or_else(|e| self.handle_mem_table_error(e));
    }

    fn delete_inner(&mut self, key: TableKey<Bytes>, value_bytes: Bytes) {
        self.local_store
            .delete(key, value_bytes)
            .unwrap_or_else(|e| self.handle_mem_table_error(e));
    }

    fn update_inner(
        &mut self,
        key_bytes: TableKey<Bytes>,
        old_value_bytes: Option<Bytes>,
        new_value_bytes: Bytes,
    ) {
        self.local_store
            .insert(key_bytes, new_value_bytes, old_value_bytes)
            .unwrap_or_else(|e| self.handle_mem_table_error(e));
    }

    /// Insert a row into state table. Must provide a full row corresponding to the column desc of
    /// the table.
    pub fn insert(&mut self, value: impl Row) {
        let pk_indices = &self.pk_indices;
        let pk = (&value).project(pk_indices);
        if USE_WATERMARK_CACHE {
            self.watermark_cache.insert(&pk);
        }

        let key_bytes = serialize_pk_with_vnode(pk, &self.pk_serde, self.compute_vnode_by_pk(pk));
        let value_bytes = self.serialize_value(value);
        self.insert_inner(key_bytes, value_bytes);
    }

    /// Delete a row from state table. Must provide a full row of old value corresponding to the
    /// column desc of the table.
    pub fn delete(&mut self, old_value: impl Row) {
        let pk_indices = &self.pk_indices;
        let pk = (&old_value).project(pk_indices);
        if USE_WATERMARK_CACHE {
            self.watermark_cache.delete(&pk);
        }

        let key_bytes = serialize_pk_with_vnode(pk, &self.pk_serde, self.compute_vnode_by_pk(pk));
        let value_bytes = self.serialize_value(old_value);
        self.delete_inner(key_bytes, value_bytes);
    }

    /// Update a row. The old and new value should have the same pk.
    pub fn update(&mut self, old_value: impl Row, new_value: impl Row) {
        let old_pk = (&old_value).project(self.pk_indices());
        let new_pk = (&new_value).project(self.pk_indices());
        debug_assert!(
            Row::eq(&old_pk, new_pk),
            "pk should not change: {old_pk:?} vs {new_pk:?}",
        );

        let new_key_bytes =
            serialize_pk_with_vnode(new_pk, &self.pk_serde, self.compute_vnode_by_pk(new_pk));
        let old_value_bytes = self.serialize_value(old_value);
        let new_value_bytes = self.serialize_value(new_value);

        self.update_inner(new_key_bytes, Some(old_value_bytes), new_value_bytes);
    }

    /// Update a row without giving old value.
    ///
    /// `op_consistency_level` should be set to `Inconsistent`.
    pub fn update_without_old_value(&mut self, new_value: impl Row) {
        let new_pk = (&new_value).project(self.pk_indices());
        let new_key_bytes =
            serialize_pk_with_vnode(new_pk, &self.pk_serde, self.compute_vnode_by_pk(new_pk));
        let new_value_bytes = self.serialize_value(new_value);

        self.update_inner(new_key_bytes, None, new_value_bytes);
    }

    /// Write a record into state table. Must have the same schema with the table.
    pub fn write_record(&mut self, record: Record<impl Row>) {
        match record {
            Record::Insert { new_row } => self.insert(new_row),
            Record::Delete { old_row } => self.delete(old_row),
            Record::Update { old_row, new_row } => self.update(old_row, new_row),
        }
    }

    fn fill_non_output_indices(&self, chunk: StreamChunk) -> StreamChunk {
        fill_non_output_indices(&self.i2o_mapping, &self.data_types, chunk)
    }

    /// Write batch with a `StreamChunk` which should have the same schema with the table.
    // allow(izip, which use zip instead of zip_eq)
    #[allow(clippy::disallowed_methods)]
    pub fn write_chunk(&mut self, chunk: StreamChunk) {
        let chunk = if IS_REPLICATED {
            self.fill_non_output_indices(chunk)
        } else {
            chunk
        };
        let (chunk, op) = chunk.into_parts();

        let vnodes = self
            .distribution
            .compute_chunk_vnode(&chunk, &self.pk_indices);

        let values = if let Some(ref value_indices) = self.value_indices {
            chunk.project(value_indices).serialize_with(&self.row_serde)
        } else {
            chunk.serialize_with(&self.row_serde)
        };

        // TODO(kwannoel): Seems like we are doing vis check twice here.
        // Once below, when using vis, and once here,
        // when using vis to set rows empty or not.
        // If we are to use the vis optimization, we should skip this.
        let key_chunk = chunk.project(self.pk_indices());
        let vnode_and_pks = key_chunk
            .rows_with_holes()
            .zip_eq_fast(vnodes.iter())
            .map(|(r, vnode)| {
                let mut buffer = BytesMut::new();
                buffer.put_slice(&vnode.to_be_bytes()[..]);
                if let Some(r) = r {
                    self.pk_serde.serialize(r, &mut buffer);
                }
                (r, buffer.freeze())
            })
            .collect_vec();

        if !key_chunk.is_compacted() {
            for ((op, (key, key_bytes), value), vis) in
                izip!(op.iter(), vnode_and_pks, values).zip_eq_debug(key_chunk.visibility().iter())
            {
                if vis {
                    match op {
                        Op::Insert | Op::UpdateInsert => {
                            if USE_WATERMARK_CACHE && let Some(ref pk) = key {
                                self.watermark_cache.insert(pk);
                            }
                            self.insert_inner(TableKey(key_bytes), value);
                        }
                        Op::Delete | Op::UpdateDelete => {
                            if USE_WATERMARK_CACHE && let Some(ref pk) = key {
                                self.watermark_cache.delete(pk);
                            }
                            self.delete_inner(TableKey(key_bytes), value);
                        }
                    }
                }
            }
        } else {
            for (op, (key, key_bytes), value) in izip!(op.iter(), vnode_and_pks, values) {
                match op {
                    Op::Insert | Op::UpdateInsert => {
                        if USE_WATERMARK_CACHE && let Some(ref pk) = key {
                            self.watermark_cache.insert(pk);
                        }
                        self.insert_inner(TableKey(key_bytes), value);
                    }
                    Op::Delete | Op::UpdateDelete => {
                        if USE_WATERMARK_CACHE && let Some(ref pk) = key {
                            self.watermark_cache.delete(pk);
                        }
                        self.delete_inner(TableKey(key_bytes), value);
                    }
                }
            }
        }
    }

    /// Update watermark for state cleaning.
    ///
    /// # Arguments
    ///
    /// * `watermark` - Latest watermark received.
    /// * `eager_cleaning` - Whether to clean up the state table eagerly.
    pub fn update_watermark(&mut self, watermark: ScalarImpl, eager_cleaning: bool) {
        trace!(table_id = %self.table_id, watermark = ?watermark, "update watermark");
        if self.watermark_buffer_strategy.apply() || eager_cleaning {
            self.state_clean_watermark = Some(watermark);
        }
    }

    pub async fn commit(&mut self, new_epoch: EpochPair) -> StreamExecutorResult<()> {
        self.commit_with_switch_consistent_op(new_epoch, None).await
    }

    pub async fn commit_with_switch_consistent_op(
        &mut self,
        new_epoch: EpochPair,
        switch_consistent_op: Option<bool>,
    ) -> StreamExecutorResult<()> {
        assert_eq!(self.epoch(), new_epoch.prev);
        let switch_op_consistency_level = switch_consistent_op.map(|enable_consistent_op| {
            assert_ne!(self.is_consistent_op, enable_consistent_op);
            self.is_consistent_op = enable_consistent_op;
            if enable_consistent_op {
                consistent_old_value_op(self.row_serde.clone())
            } else {
                OpConsistencyLevel::Inconsistent
            }
        });
        trace!(
            table_id = %self.table_id,
            epoch = ?self.epoch(),
            "commit state table"
        );
        // Tick the watermark buffer here because state table is expected to be committed once
        // per epoch.
        self.watermark_buffer_strategy.tick();
        if !self.is_dirty() {
            // If the state table is not modified, go fast path.
            self.local_store.seal_current_epoch(
                new_epoch.curr,
                SealCurrentEpochOptions {
                    table_watermarks: None,
                    switch_op_consistency_level,
                },
            );
            return Ok(());
        } else {
            self.seal_current_epoch(new_epoch.curr, switch_op_consistency_level)
                .instrument(tracing::info_span!("state_table_commit"))
                .await?;
        }

        // Refresh watermark cache if it is out of sync.
        if USE_WATERMARK_CACHE && !self.watermark_cache.is_synced() {
            if let Some(ref watermark) = self.prev_cleaned_watermark {
                let range: (Bound<Once<Datum>>, Bound<Once<Datum>>) =
                    (Included(once(Some(watermark.clone()))), Unbounded);
                // NOTE(kwannoel): We buffer `pks` before inserting into watermark cache
                // because we can't hold an immutable ref (via `iter_key_and_val_with_pk_range`)
                // and a mutable ref (via `self.watermark_cache.insert`) at the same time.
                // TODO(kwannoel): We can optimize it with:
                // 1. Either use `RefCell`.
                // 2. Or pass in a direct reference to LocalStateStore,
                //    instead of referencing it indirectly from `self`.
                //    Similar to how we do for pk_indices.
                let mut pks = Vec::with_capacity(self.watermark_cache.capacity());
                {
                    let mut streams = vec![];
                    for vnode in self.vnodes().iter_vnodes() {
                        let stream = self
                            .iter_with_vnode(vnode, &range, PrefetchOptions::default())
                            .await?;
                        streams.push(Box::pin(stream));
                    }
                    let merged_stream = merge_sort(streams);
                    pin_mut!(merged_stream);

                    #[for_await]
                    for entry in merged_stream.take(self.watermark_cache.capacity()) {
                        let keyed_row = entry?;
                        let pk = self.pk_serde.deserialize(keyed_row.key())?;
                        if !pk.is_null_at(0) {
                            pks.push(pk);
                        }
                    }
                }

                let mut filler = self.watermark_cache.begin_syncing();
                for pk in pks {
                    filler.insert_unchecked(DefaultOrdered(pk), ());
                }
                filler.finish();

                let n_cache_entries = self.watermark_cache.len();
                if n_cache_entries < self.watermark_cache.capacity() {
                    self.watermark_cache.set_table_row_count(n_cache_entries);
                }
            }
        }

        Ok(())
    }

    // TODO(st1page): maybe we should extract a pub struct to do it
    /// just specially used by those state table read-only and after the call the data
    /// in the epoch will be visible
    pub fn commit_no_data_expected(&mut self, new_epoch: EpochPair) {
        assert_eq!(self.epoch(), new_epoch.prev);
        assert!(!self.is_dirty());
        // Tick the watermark buffer here because state table is expected to be committed once
        // per epoch.
        self.watermark_buffer_strategy.tick();
        self.local_store.seal_current_epoch(
            new_epoch.curr,
            SealCurrentEpochOptions {
                table_watermarks: None,
                switch_op_consistency_level: None,
            },
        );
    }

    /// Write to state store.
    async fn seal_current_epoch(
        &mut self,
        next_epoch: u64,
        switch_op_consistency_level: Option<OpConsistencyLevel>,
    ) -> StreamExecutorResult<()> {
        let watermark = self.state_clean_watermark.take();
        watermark.as_ref().inspect(|watermark| {
            trace!(table_id = %self.table_id, watermark = ?watermark, "state cleaning");
        });

        let prefix_serializer = if self.pk_indices().is_empty() {
            None
        } else {
            Some(self.pk_serde.prefix(1))
        };

        let should_clean_watermark = match watermark {
            Some(ref watermark) => {
                if USE_WATERMARK_CACHE && self.watermark_cache.is_synced() {
                    if let Some(key) = self.watermark_cache.lowest_key() {
                        watermark.as_scalar_ref_impl().default_cmp(&key).is_ge()
                    } else {
                        // Watermark cache is synced,
                        // And there's no key in watermark cache.
                        // That implies table is empty.
                        // We should not clean watermark.
                        false
                    }
                } else {
                    // Either we are not using watermark cache,
                    // Or watermark_cache is not synced.
                    // In either case we should clean watermark.
                    true
                }
            }
            None => false,
        };

        let watermark_suffix = watermark.as_ref().map(|watermark| {
            serialize_pk(
                row::once(Some(watermark.clone())),
                prefix_serializer.as_ref().unwrap(),
            )
        });

        let mut seal_watermark: Option<(WatermarkDirection, VnodeWatermark)> = None;

        // Compute Delete Ranges
        if should_clean_watermark && let Some(watermark_suffix) = watermark_suffix {
            trace!(table_id = %self.table_id, watermark = ?watermark_suffix, vnodes = ?{
                self.vnodes().iter_vnodes().collect_vec()
            }, "delete range");
            if prefix_serializer
                .as_ref()
                .unwrap()
                .get_order_types()
                .first()
                .unwrap()
                .is_ascending()
            {
                seal_watermark = Some((
                    WatermarkDirection::Ascending,
                    VnodeWatermark::new(
                        self.vnodes().clone(),
                        Bytes::copy_from_slice(watermark_suffix.as_ref()),
                    ),
                ));
            } else {
                seal_watermark = Some((
                    WatermarkDirection::Descending,
                    VnodeWatermark::new(
                        self.vnodes().clone(),
                        Bytes::copy_from_slice(watermark_suffix.as_ref()),
                    ),
                ));
            }
        }
        self.prev_cleaned_watermark = watermark;

        // Clear the watermark cache and force a resync.
        // TODO(kwannoel): This can be further optimized:
        // 1. Add a `cache.drain_until` interface, so we only clear the watermark cache
        //    up to the largest end of delete ranges.
        // 2. Mark the cache as not_synced, so we can still refill it later.
        // 3. When refilling the cache,
        //    we just refill from the largest value of the cache, as the lower bound.
        if USE_WATERMARK_CACHE && seal_watermark.is_some() {
            self.watermark_cache.clear();
        }

        self.local_store.flush(vec![]).await?;
        let table_watermarks =
            seal_watermark.map(|(direction, watermark)| (direction, vec![watermark]));

        self.local_store.seal_current_epoch(
            next_epoch,
            SealCurrentEpochOptions {
                table_watermarks,
                switch_op_consistency_level,
            },
        );
        Ok(())
    }

    pub async fn try_flush(&mut self) -> StreamExecutorResult<()> {
        self.local_store.try_flush().await?;
        Ok(())
    }
}

// Iterator functions
impl<
        S,
        SD,
        const IS_REPLICATED: bool,
        W: WatermarkBufferStrategy,
        const USE_WATERMARK_CACHE: bool,
    > StateTableInner<S, SD, IS_REPLICATED, W, USE_WATERMARK_CACHE>
where
    S: StateStore,
    SD: ValueRowSerde,
{
    /// This function scans rows from the relational table with specific `pk_range` under the same
    /// `vnode`.
    pub async fn iter_with_vnode(
        &self,

        // Optional vnode that returns an iterator only over the given range under that vnode.
        // For now, we require this parameter, and will panic. In the future, when `None`, we can
        // iterate over each vnode that the `StateTableInner` owns.
        vnode: VirtualNode,
        pk_range: &(Bound<impl Row>, Bound<impl Row>),
        prefetch_options: PrefetchOptions,
    ) -> StreamExecutorResult<KeyedRowStream<'_, S, SD>> {
        Ok(deserialize_keyed_row_stream(
            self.iter_kv_with_pk_range(pk_range, vnode, prefetch_options)
                .await?,
            &self.row_serde,
        ))
    }

    pub async fn iter_with_vnode_and_output_indices(
        &self,
        vnode: VirtualNode,
        pk_range: &(Bound<impl Row>, Bound<impl Row>),
        prefetch_options: PrefetchOptions,
    ) -> StreamExecutorResult<impl Stream<Item = StreamExecutorResult<KeyedRow<Bytes>>> + '_> {
        assert!(IS_REPLICATED);
        let stream = self
            .iter_with_vnode(vnode, pk_range, prefetch_options)
            .await?;
        Ok(stream.map(|row| {
            row.map(|keyed_row| {
                let (vnode_prefixed_key, row) = keyed_row.into_parts();
                let row = row.project(&self.output_indices).into_owned_row();
                KeyedRow::new(vnode_prefixed_key, row)
            })
        }))
    }

    async fn iter_kv(
        &self,
        table_key_range: TableKeyRange,
        prefix_hint: Option<Bytes>,
        prefetch_options: PrefetchOptions,
    ) -> StreamExecutorResult<<S::Local as LocalStateStore>::IterStream<'_>> {
        let read_options = ReadOptions {
            prefix_hint,
            retention_seconds: self.table_option.retention_seconds,
            table_id: self.table_id,
            prefetch_options,
            cache_policy: CachePolicy::Fill(CachePriority::High),
            ..Default::default()
        };

        Ok(self.local_store.iter(table_key_range, read_options).await?)
    }

    /// This function scans rows from the relational table with specific `prefix` and `sub_range` under the same
    /// `vnode`. If `sub_range` is (Unbounded, Unbounded), it scans rows from the relational table with specific `pk_prefix`.
    /// `pk_prefix` is used to identify the exact vnode the scan should perform on.

    /// This function scans rows from the relational table with specific `prefix` and `pk_sub_range` under the same
    /// `vnode`.
    pub async fn iter_with_prefix(
        &self,
        pk_prefix: impl Row,
        sub_range: &(Bound<impl Row>, Bound<impl Row>),
        prefetch_options: PrefetchOptions,
    ) -> StreamExecutorResult<KeyedRowStream<'_, S, SD>> {
        let prefix_serializer = self.pk_serde.prefix(pk_prefix.len());
        let encoded_prefix = serialize_pk(&pk_prefix, &prefix_serializer);

        // We assume that all usages of iterating the state table only access a single vnode.
        // If this assertion fails, then something must be wrong with the operator implementation or
        // the distribution derivation from the optimizer.
        let vnode = self.compute_prefix_vnode(&pk_prefix);

        // Construct prefix hint for prefix bloom filter.
        let pk_prefix_indices = &self.pk_indices[..pk_prefix.len()];
        if self.prefix_hint_len != 0 {
            debug_assert_eq!(self.prefix_hint_len, pk_prefix.len());
        }
        let prefix_hint = {
            if self.prefix_hint_len == 0 || self.prefix_hint_len > pk_prefix.len() {
                None
            } else {
                let encoded_prefix_len = self
                    .pk_serde
                    .deserialize_prefix_len(&encoded_prefix, self.prefix_hint_len)?;

                Some(Bytes::copy_from_slice(
                    &encoded_prefix[..encoded_prefix_len],
                ))
            }
        };

        trace!(
            table_id = %self.table_id(),
            ?prefix_hint, ?pk_prefix,
             ?pk_prefix_indices,
            "storage_iter_with_prefix"
        );

        let memcomparable_range =
            prefix_and_sub_range_to_memcomparable(&self.pk_serde, sub_range, pk_prefix);

        let memcomparable_range_with_vnode = prefixed_range_with_vnode(memcomparable_range, vnode);

        Ok(deserialize_keyed_row_stream(
            self.iter_kv(
                memcomparable_range_with_vnode,
                prefix_hint,
                prefetch_options,
            )
            .await?,
            &self.row_serde,
        ))
    }

    /// This function scans raw key-values from the relational table with specific `pk_range` under
    /// the same `vnode`.
    async fn iter_kv_with_pk_range(
        &self,
        pk_range: &(Bound<impl Row>, Bound<impl Row>),
        // Optional vnode that returns an iterator only over the given range under that vnode.
        // For now, we require this parameter, and will panic. In the future, when `None`, we can
        // iterate over each vnode that the `StateTableInner` owns.
        vnode: VirtualNode,
        prefetch_options: PrefetchOptions,
    ) -> StreamExecutorResult<<S::Local as LocalStateStore>::IterStream<'_>> {
        let memcomparable_range = prefix_range_to_memcomparable(&self.pk_serde, pk_range);
        let memcomparable_range_with_vnode = prefixed_range_with_vnode(memcomparable_range, vnode);

        // TODO: provide a trace of useful params.
        self.iter_kv(memcomparable_range_with_vnode, None, prefetch_options)
            .await
            .map_err(StreamExecutorError::from)
    }

    /// Returns:
    /// false: the provided pk prefix is absent in state store.
    /// true: the provided pk prefix may or may not be present in state store.
    pub async fn may_exist(&self, pk_prefix: impl Row) -> StreamExecutorResult<bool> {
        let prefix_serializer = self.pk_serde.prefix(pk_prefix.len());
        let encoded_prefix = serialize_pk(&pk_prefix, &prefix_serializer);
        let encoded_key_range = range_of_prefix(&encoded_prefix);

        // We assume that all usages of iterating the state table only access a single vnode.
        // If this assertion fails, then something must be wrong with the operator implementation or
        // the distribution derivation from the optimizer.
        let vnode = self.compute_prefix_vnode(&pk_prefix);
        let table_key_range = prefixed_range_with_vnode(encoded_key_range, vnode);

        // Construct prefix hint for prefix bloom filter.
        if self.prefix_hint_len != 0 {
            debug_assert_eq!(self.prefix_hint_len, pk_prefix.len());
        }
        let prefix_hint = {
            if self.prefix_hint_len == 0 || self.prefix_hint_len > pk_prefix.len() {
                panic!();
            } else {
                let encoded_prefix_len = self
                    .pk_serde
                    .deserialize_prefix_len(&encoded_prefix, self.prefix_hint_len)?;

                Some(Bytes::copy_from_slice(
                    &encoded_prefix[..encoded_prefix_len],
                ))
            }
        };

        let read_options = ReadOptions {
            prefix_hint,
            table_id: self.table_id,
            cache_policy: CachePolicy::Fill(CachePriority::High),
            ..Default::default()
        };

        self.local_store
            .may_exist(table_key_range, read_options)
            .await
            .map_err(Into::into)
    }

    #[cfg(test)]
    pub fn get_watermark_cache(&self) -> &StateTableWatermarkCache {
        &self.watermark_cache
    }
}

pub type KeyedRowStream<'a, S: StateStore, SD: ValueRowSerde + 'a> =
    impl Stream<Item = StreamExecutorResult<KeyedRow<Bytes>>> + 'a;

fn deserialize_keyed_row_stream<'a>(
    stream: impl StateStoreIterItemStream + 'a,
    deserializer: &'a impl ValueRowSerde,
) -> impl Stream<Item = StreamExecutorResult<KeyedRow<Bytes>>> + 'a {
    stream.map(move |result| {
        result
            .map_err(StreamExecutorError::from)
            .and_then(|(key, value)| {
                Ok(KeyedRow::new(
                    key.user_key.table_key,
                    deserializer.deserialize(&value).map(OwnedRow::new)?,
                ))
            })
    })
}

pub fn prefix_range_to_memcomparable(
    pk_serde: &OrderedRowSerde,
    range: &(Bound<impl Row>, Bound<impl Row>),
) -> (Bound<Bytes>, Bound<Bytes>) {
    (
        start_range_to_memcomparable(pk_serde, &range.0),
        end_range_to_memcomparable(pk_serde, &range.1, None),
    )
}

fn prefix_and_sub_range_to_memcomparable(
    pk_serde: &OrderedRowSerde,
    sub_range: &(Bound<impl Row>, Bound<impl Row>),
    pk_prefix: impl Row,
) -> (Bound<Bytes>, Bound<Bytes>) {
    let (range_start, range_end) = sub_range;
    let prefix_serializer = pk_serde.prefix(pk_prefix.len());
    let serialized_pk_prefix = serialize_pk(&pk_prefix, &prefix_serializer);
    let start_range = match range_start {
        Included(start_range) => Bound::Included(Either::Left((&pk_prefix).chain(start_range))),
        Excluded(start_range) => Bound::Excluded(Either::Left((&pk_prefix).chain(start_range))),
        Unbounded => Bound::Included(Either::Right(&pk_prefix)),
    };
    let end_range = match range_end {
        Included(end_range) => Bound::Included((&pk_prefix).chain(end_range)),
        Excluded(end_range) => Bound::Excluded((&pk_prefix).chain(end_range)),
        Unbounded => Unbounded,
    };
    (
        start_range_to_memcomparable(pk_serde, &start_range),
        end_range_to_memcomparable(pk_serde, &end_range, Some(serialized_pk_prefix)),
    )
}

fn start_range_to_memcomparable<R: Row>(
    pk_serde: &OrderedRowSerde,
    bound: &Bound<R>,
) -> Bound<Bytes> {
    let serialize_pk_prefix = |pk_prefix: &R| {
        let prefix_serializer = pk_serde.prefix(pk_prefix.len());
        serialize_pk(pk_prefix, &prefix_serializer)
    };
    match bound {
        Unbounded => Unbounded,
        Included(r) => {
            let serialized = serialize_pk_prefix(r);

            Included(serialized)
        }
        Excluded(r) => {
            let serialized = serialize_pk_prefix(r);

            start_bound_of_excluded_prefix(&serialized)
        }
    }
}

fn end_range_to_memcomparable<R: Row>(
    pk_serde: &OrderedRowSerde,
    bound: &Bound<R>,
    serialized_pk_prefix: Option<Bytes>,
) -> Bound<Bytes> {
    let serialize_pk_prefix = |pk_prefix: &R| {
        let prefix_serializer = pk_serde.prefix(pk_prefix.len());
        serialize_pk(pk_prefix, &prefix_serializer)
    };
    match bound {
        Unbounded => match serialized_pk_prefix {
            Some(serialized_pk_prefix) => end_bound_of_prefix(&serialized_pk_prefix),
            None => Unbounded,
        },
        Included(r) => {
            let serialized = serialize_pk_prefix(r);

            end_bound_of_prefix(&serialized)
        }
        Excluded(r) => {
            let serialized = serialize_pk_prefix(r);
            Excluded(serialized)
        }
    }
}

fn fill_non_output_indices(
    i2o_mapping: &ColIndexMapping,
    data_types: &[DataType],
    chunk: StreamChunk,
) -> StreamChunk {
    let cardinality = chunk.cardinality();
    let (ops, columns, vis) = chunk.into_inner();
    let mut full_columns = Vec::with_capacity(data_types.len());
    for (i, data_type) in data_types.iter().enumerate() {
        if let Some(j) = i2o_mapping.try_map(i) {
            full_columns.push(columns[j].clone());
        } else {
            let mut column_builder = ArrayImplBuilder::with_type(cardinality, data_type.clone());
            column_builder.append_n_null(cardinality);
            let column: ArrayRef = column_builder.finish().into();
            full_columns.push(column)
        }
    }
    let data_chunk = DataChunk::new(full_columns, vis);
    StreamChunk::from_parts(ops, data_chunk)
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use expect_test::{expect, Expect};

    use super::*;

    fn check(actual: impl Debug, expect: Expect) {
        let actual = format!("{:#?}", actual);
        expect.assert_eq(&actual);
    }

    #[test]
    fn test_fill_non_output_indices() {
        let data_types = vec![DataType::Int32, DataType::Int32, DataType::Int32];
        let replicated_chunk = [OwnedRow::new(vec![
            Some(222_i32.into()),
            Some(2_i32.into()),
        ])];
        let replicated_chunk = StreamChunk::from_parts(
            vec![Op::Insert],
            DataChunk::from_rows(&replicated_chunk, &[DataType::Int32, DataType::Int32]),
        );
        let i2o_mapping = ColIndexMapping::new(vec![Some(1), None, Some(0)], 2);
        let filled_chunk = fill_non_output_indices(&i2o_mapping, &data_types, replicated_chunk);
        check(
            filled_chunk,
            expect![[r#"
            StreamChunk { cardinality: 1, capacity: 1, data:
            +---+---+---+-----+
            | + | 2 |   | 222 |
            +---+---+---+-----+
             }"#]],
        );
    }
}
