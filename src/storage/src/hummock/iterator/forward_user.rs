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

use std::ops::Bound::*;

use bytes::Bytes;
use risingwave_common::util::epoch::MAX_SPILL_TIMES;
use risingwave_hummock_sdk::key::{FullKey, UserKey, UserKeyRange};
use risingwave_hummock_sdk::{EpochWithGap, HummockEpoch};

use super::DeleteRangeIterator;
use crate::hummock::iterator::{Forward, ForwardMergeRangeIterator, HummockIterator};
use crate::hummock::local_version::pinned_version::PinnedVersion;
use crate::hummock::value::HummockValue;
use crate::hummock::HummockResult;
use crate::monitor::StoreLocalStatistic;

/// [`UserIterator`] can be used by user directly.
pub struct UserIterator<I: HummockIterator<Direction = Forward>> {
    /// Inner table iterator.
    iterator: I,

    /// Last full key.
    last_key: FullKey<Bytes>,

    /// Last user value
    last_val: Bytes,

    /// Start and end bounds of user key.
    key_range: UserKeyRange,

    /// Only reads values if `ts <= self.read_epoch`.
    read_epoch: HummockEpoch,

    /// Only reads values if `ts > self.min_epoch`. use for ttl
    min_epoch: HummockEpoch,

    /// Ensures the SSTs needed by `iterator` won't be vacuumed.
    _version: Option<PinnedVersion>,

    stats: StoreLocalStatistic,

    delete_range_iter: ForwardMergeRangeIterator,

    /// Whether the iterator is pointing to a valid position
    is_current_pos_valid: bool,
}

// TODO: decide whether this should also impl `HummockIterator`
impl<I: HummockIterator<Direction = Forward>> UserIterator<I> {
    /// Create [`UserIterator`] with given `read_epoch`.
    pub(crate) fn new(
        iterator: I,
        key_range: UserKeyRange,
        read_epoch: u64,
        min_epoch: u64,
        version: Option<PinnedVersion>,
        delete_range_iter: ForwardMergeRangeIterator,
    ) -> Self {
        Self {
            iterator,
            key_range,
            last_key: FullKey::default(),
            last_val: Bytes::new(),
            read_epoch,
            min_epoch,
            stats: StoreLocalStatistic::default(),
            delete_range_iter,
            _version: version,
            is_current_pos_valid: false,
        }
    }

    /// Create [`UserIterator`] with maximum epoch.
    pub fn for_test(iterator: I, key_range: UserKeyRange) -> Self {
        let read_epoch = HummockEpoch::MAX;
        Self::new(
            iterator,
            key_range,
            read_epoch,
            0,
            None,
            ForwardMergeRangeIterator::new(read_epoch),
        )
    }

    /// Gets the iterator move to the next step.
    /// See `IteratorState` for the state machine details.
    ///
    /// Returned result:
    /// - if `Ok(())` is returned, it means that the iterator successfully move to the next position
    ///   (may reach to the end and thus not valid)
    /// - if `Err(_) ` is returned, it means that some error happened.
    pub async fn next(&mut self) -> HummockResult<()> {
        // Reset the valid flag to make sure if error happens, `is_valid` should return false.
        self.is_current_pos_valid = false;
        // Move the iterator to the next step if it is currently potined to a ready entry.
        self.iterator.next().await?;

        // Check and move onto the next valid position if any
        self.try_advance_to_next_valid().await
    }

    /// Returns the key with the newest version. Thus no version in it, and only the `user_key` will
    /// be returned.
    ///
    /// The returned key is de-duplicated and thus it will not output the same key, unless the
    /// `rewind` or `seek` methods are called.
    ///
    /// Note: before call the function you need to ensure that the iterator is valid.
    pub fn key(&self) -> &FullKey<Bytes> {
        assert!(self.is_valid());
        &self.last_key
    }

    /// The returned value is in the form of user value.
    ///
    /// Note: before call the function you need to ensure that the iterator is valid.
    pub fn value(&self) -> &Bytes {
        assert!(self.is_valid());
        &self.last_val
    }

    /// Resets the iterating position to the beginning.
    pub async fn rewind(&mut self) -> HummockResult<()> {
        // Reset
        self.is_current_pos_valid = false;
        self.last_key = FullKey::default();

        // Handle range scan
        match &self.key_range.0 {
            Included(begin_key) => {
                let full_key = FullKey {
                    user_key: begin_key.clone(),
                    epoch_with_gap: EpochWithGap::new(self.read_epoch, MAX_SPILL_TIMES),
                };
                self.iterator.seek(full_key.to_ref()).await?;
                // We check `is_valid` and `user_key_out_of_range` here to avoid `delete_range_iter.seek`
                // as much as possible because it can be expensive. We can simplify this logic after delete
                // range is deprecated and replaced with vnode watermark.
                if !self.iterator.is_valid() {
                    return Ok(());
                }

                if self.user_key_out_of_range(self.iterator.key().user_key) {
                    return Ok(());
                }

                self.delete_range_iter.seek(begin_key.as_ref()).await?;
            }
            Excluded(_) => unimplemented!("excluded begin key is not supported"),
            Unbounded => {
                // We check `is_valid` here to avoid `delete_range_iter.rewind`
                // as much as possible because it can be expensive. We can simplify this logic after delete
                // range is deprecated and replaced with vnode watermark.
                self.iterator.rewind().await?;
                if !self.iterator.is_valid() {
                    return Ok(());
                }

                self.delete_range_iter.rewind().await?;
            }
        };

        self.try_advance_to_next_valid().await
    }

    /// Resets the iterating position to the first position where the key >= provided key.
    pub async fn seek(&mut self, user_key: UserKey<&[u8]>) -> HummockResult<()> {
        // Reset
        self.is_current_pos_valid = false;
        self.last_key = FullKey::default();

        // Handle range scan when key < begin_key
        let user_key = match &self.key_range.0 {
            Included(begin_key) => {
                let begin_key = begin_key.as_ref();
                if begin_key > user_key {
                    begin_key
                } else {
                    user_key
                }
            }
            Excluded(_) => unimplemented!("excluded begin key is not supported"),
            Unbounded => user_key,
        };

        let full_key = FullKey {
            user_key,
            epoch_with_gap: EpochWithGap::new(self.read_epoch, MAX_SPILL_TIMES),
        };
        self.iterator.seek(full_key).await?;
        // We check `is_valid` and `user_key_out_of_range` here to avoid `delete_range_iter.seek`
        // as much as possible because it can be expensive. We can simplify this logic after delete
        // range is deprecated and replaced with vnode watermark.
        if !self.iterator.is_valid() {
            return Ok(());
        }

        if self.user_key_out_of_range(self.iterator.key().user_key) {
            return Ok(());
        }

        self.delete_range_iter.seek(full_key.user_key).await?;

        self.try_advance_to_next_valid().await
    }

    /// Indicates whether the iterator can be used.
    pub fn is_valid(&self) -> bool {
        self.is_current_pos_valid
    }

    pub fn collect_local_statistic(&self, stats: &mut StoreLocalStatistic) {
        stats.add(&self.stats);
        self.iterator.collect_local_statistic(stats);
    }

    /// Advance the inner iterator to a valid position, in which the entry can be exposed.
    /// Iterator will not be advanced if it already pointed to a valid position.
    async fn try_advance_to_next_valid(&mut self) -> HummockResult<()> {
        loop {
            if !self.iterator.is_valid() {
                break;
            }

            let full_key = self.iterator.key();
            let epoch = full_key.epoch_with_gap.pure_epoch();

            // Handle epoch visibility
            if epoch < self.min_epoch || epoch > self.read_epoch {
                self.iterator.next().await?;
                continue;
            }

            // Skip older version entry for the same user key
            if self.last_key.user_key.as_ref() == full_key.user_key {
                self.stats.skip_multi_version_key_count += 1;
                self.iterator.next().await?;
                continue;
            }

            // A new user key is observed.
            self.last_key = full_key.copy_into();

            // It is better to early return here if the user key is already
            // out of range to avoid unnecessary access on the range tomestones
            // via `delete_range_iter`.
            // For example, if we are iterating with key range [0x0a, 0x0c) and the
            // current key is 0xff, we will access range tombstones in [0x0c, 0xff],
            // which is a waste of work.
            if self.user_key_out_of_range(full_key.user_key) {
                break;
            }

            // Handle delete operation
            match self.iterator.value() {
                HummockValue::Put(val) => {
                    self.delete_range_iter.next_until(full_key.user_key).await?;
                    if self.delete_range_iter.current_epoch() >= epoch {
                        self.stats.skip_delete_key_count += 1;
                    } else {
                        self.last_val = Bytes::copy_from_slice(val);
                        self.stats.processed_key_count += 1;
                        self.is_current_pos_valid = true;
                        return Ok(());
                    }
                }
                // It means that the key is deleted from the storage.
                // Deleted kv and the previous versions (if any) of the key should not be
                // returned to user.
                HummockValue::Delete => {
                    self.stats.skip_delete_key_count += 1;
                }
            }
            self.iterator.next().await?;
        }

        self.is_current_pos_valid = false;
        Ok(())
    }

    // Validate whether the current key is already out of range.
    fn user_key_out_of_range(&self, user_key: UserKey<&[u8]>) -> bool {
        // handle range scan
        match &self.key_range.1 {
            Included(end_key) => user_key > end_key.as_ref(),
            Excluded(end_key) => user_key >= end_key.as_ref(),
            Unbounded => false,
        }
    }
}

#[cfg(test)]
impl<I: HummockIterator<Direction = Forward>> UserIterator<I> {
    pub(crate) fn for_test_with_epoch(
        iterator: I,
        key_range: UserKeyRange,
        read_epoch: u64,
        min_epoch: u64,
    ) -> Self {
        Self::new(
            iterator,
            key_range,
            read_epoch,
            min_epoch,
            None,
            ForwardMergeRangeIterator::new(read_epoch),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound::*;
    use std::sync::Arc;

    use super::*;
    use crate::hummock::iterator::test_utils::{
        default_builder_opt_for_test, gen_iterator_test_sstable_base,
        gen_iterator_test_sstable_from_kv_pair, gen_iterator_test_sstable_with_incr_epoch,
        gen_iterator_test_sstable_with_range_tombstones, iterator_test_bytes_key_of,
        iterator_test_bytes_key_of_epoch, iterator_test_bytes_user_key_of, iterator_test_value_of,
        mock_sstable_store, TEST_KEYS_COUNT,
    };
    use crate::hummock::iterator::MergeIterator;
    use crate::hummock::sstable::{
        SstableIterator, SstableIteratorReadOptions, SstableIteratorType,
    };
    use crate::hummock::sstable_store::SstableStoreRef;
    use crate::hummock::value::HummockValue;
    use crate::hummock::{SstableDeleteRangeIterator, TableHolder};

    #[tokio::test]
    async fn test_basic() {
        let sstable_store = mock_sstable_store();
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let table0 = gen_iterator_test_sstable_base(
            0,
            default_builder_opt_for_test(),
            |x| x * 3,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let table1 = gen_iterator_test_sstable_base(
            1,
            default_builder_opt_for_test(),
            |x| x * 3 + 1,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let table2 = gen_iterator_test_sstable_base(
            2,
            default_builder_opt_for_test(),
            |x| x * 3 + 2,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let iters = vec![
            SstableIterator::create(table0, sstable_store.clone(), read_options.clone()),
            SstableIterator::create(table1, sstable_store.clone(), read_options.clone()),
            SstableIterator::create(table2, sstable_store, read_options.clone()),
        ];

        let mi = MergeIterator::new(iters);
        let mut ui = UserIterator::for_test(mi, (Unbounded, Unbounded));
        ui.rewind().await.unwrap();

        let mut i = 0;
        while ui.is_valid() {
            let key = ui.key();
            let val = ui.value();
            assert_eq!(key, &iterator_test_bytes_key_of(i));
            assert_eq!(val, iterator_test_value_of(i).as_slice());
            i += 1;
            ui.next().await.unwrap();
            if i == TEST_KEYS_COUNT * 3 {
                assert!(!ui.is_valid());
                break;
            }
        }
        assert!(i >= TEST_KEYS_COUNT * 3);
    }

    #[tokio::test]
    async fn test_seek() {
        let sstable_store = mock_sstable_store();
        let table0 = gen_iterator_test_sstable_base(
            0,
            default_builder_opt_for_test(),
            |x| x * 3,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let table1 = gen_iterator_test_sstable_base(
            1,
            default_builder_opt_for_test(),
            |x| x * 3 + 1,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let table2 = gen_iterator_test_sstable_base(
            2,
            default_builder_opt_for_test(),
            |x| x * 3 + 2,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
        )
        .await;
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![
            SstableIterator::create(table0, sstable_store.clone(), read_options.clone()),
            SstableIterator::create(table1, sstable_store.clone(), read_options.clone()),
            SstableIterator::create(table2, sstable_store, read_options),
        ];

        let mi = MergeIterator::new(iters);
        let mut ui = UserIterator::for_test(mi, (Unbounded, Unbounded));

        // right edge case
        ui.seek(iterator_test_bytes_user_key_of(3 * TEST_KEYS_COUNT).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());

        // normal case
        ui.seek(iterator_test_bytes_user_key_of(TEST_KEYS_COUNT + 5).as_ref())
            .await
            .unwrap();
        let k = ui.key();
        let v = ui.value();
        assert_eq!(v, iterator_test_value_of(TEST_KEYS_COUNT + 5).as_slice());
        assert_eq!(k, &iterator_test_bytes_key_of(TEST_KEYS_COUNT + 5));
        ui.seek(iterator_test_bytes_user_key_of(2 * TEST_KEYS_COUNT + 5).as_ref())
            .await
            .unwrap();
        let k = ui.key();
        let v = ui.value();
        assert_eq!(
            v,
            iterator_test_value_of(2 * TEST_KEYS_COUNT + 5).as_slice()
        );
        assert_eq!(k, &iterator_test_bytes_key_of(2 * TEST_KEYS_COUNT + 5));

        // left edge case
        ui.seek(iterator_test_bytes_user_key_of(0).as_ref())
            .await
            .unwrap();
        let k = ui.key();
        let v = ui.value();
        assert_eq!(v, iterator_test_value_of(0).as_slice());
        assert_eq!(k, &iterator_test_bytes_key_of(0));
    }

    #[tokio::test]
    async fn test_delete() {
        let sstable_store = mock_sstable_store();

        // key=[idx, epoch], value
        let kv_pairs = vec![
            (1, 100, HummockValue::put(iterator_test_value_of(1))),
            (2, 300, HummockValue::delete()),
        ];
        let table0 =
            gen_iterator_test_sstable_from_kv_pair(0, kv_pairs, sstable_store.clone()).await;

        let kv_pairs = vec![
            (1, 200, HummockValue::delete()),
            (2, 400, HummockValue::put(iterator_test_value_of(2))),
        ];
        let table1 =
            gen_iterator_test_sstable_from_kv_pair(1, kv_pairs, sstable_store.clone()).await;

        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![
            SstableIterator::create(table0, sstable_store.clone(), read_options.clone()),
            SstableIterator::create(table1, sstable_store.clone(), read_options),
        ];

        let mi = MergeIterator::new(iters);
        let mut ui = UserIterator::for_test(mi, (Unbounded, Unbounded));
        ui.rewind().await.unwrap();

        // verify
        let k = ui.key();
        let v = ui.value();
        assert_eq!(k, &iterator_test_bytes_key_of_epoch(2, 400));
        assert_eq!(v, &Bytes::from(iterator_test_value_of(2)));

        // only one valid kv pair
        ui.next().await.unwrap();
        assert!(!ui.is_valid());
    }

    async fn generate_test_data(
        sstable_store: SstableStoreRef,
        range_tombstones: Vec<(usize, usize, u64)>,
    ) -> TableHolder {
        let kv_pairs = vec![
            (0, 200, HummockValue::delete()),
            (0, 100, HummockValue::put(iterator_test_value_of(0))),
            (1, 200, HummockValue::put(iterator_test_value_of(1))),
            (1, 100, HummockValue::delete()),
            (2, 300, HummockValue::put(iterator_test_value_of(2))),
            (2, 200, HummockValue::delete()),
            (2, 100, HummockValue::delete()),
            (3, 100, HummockValue::put(iterator_test_value_of(3))),
            (5, 200, HummockValue::delete()),
            (5, 100, HummockValue::put(iterator_test_value_of(5))),
            (6, 100, HummockValue::put(iterator_test_value_of(6))),
            (7, 200, HummockValue::delete()),
            (7, 100, HummockValue::put(iterator_test_value_of(7))),
            (8, 100, HummockValue::put(iterator_test_value_of(8))),
        ];
        let sst_info = gen_iterator_test_sstable_with_range_tombstones(
            0,
            kv_pairs,
            range_tombstones,
            sstable_store.clone(),
        )
        .await;
        sstable_store
            .sstable(&sst_info, &mut StoreLocalStatistic::default())
            .await
            .unwrap()
    }

    // left..=end
    #[tokio::test]
    async fn test_range_inclusive() {
        let sstable_store = mock_sstable_store();
        // key=[idx, epoch], value
        let table = generate_test_data(sstable_store.clone(), vec![]).await;
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![SstableIterator::create(table, sstable_store, read_options)];
        let mi = MergeIterator::new(iters);

        let begin_key = Included(iterator_test_bytes_user_key_of(2));
        let end_key = Included(iterator_test_bytes_user_key_of(7));

        let mut ui = UserIterator::for_test(mi, (begin_key, end_key));

        // ----- basic iterate -----
        ui.rewind().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- before-begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(1).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(2).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(7).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());

        // ----- after-end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(8).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());
    }

    // left..end
    #[tokio::test]
    async fn test_range() {
        let sstable_store = mock_sstable_store();
        // key=[idx, epoch], value
        let kv_pairs = vec![
            (0, 200, HummockValue::delete()),
            (0, 100, HummockValue::put(iterator_test_value_of(0))),
            (1, 200, HummockValue::put(iterator_test_value_of(1))),
            (1, 100, HummockValue::delete()),
            (2, 300, HummockValue::put(iterator_test_value_of(2))),
            (2, 200, HummockValue::delete()),
            (2, 100, HummockValue::delete()),
            (3, 100, HummockValue::put(iterator_test_value_of(3))),
            (5, 200, HummockValue::delete()),
            (5, 100, HummockValue::put(iterator_test_value_of(5))),
            (6, 100, HummockValue::put(iterator_test_value_of(6))),
            (7, 100, HummockValue::put(iterator_test_value_of(7))),
            (8, 100, HummockValue::put(iterator_test_value_of(8))),
        ];
        let table =
            gen_iterator_test_sstable_from_kv_pair(0, kv_pairs, sstable_store.clone()).await;
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![SstableIterator::create(table, sstable_store, read_options)];
        let mi = MergeIterator::new(iters);

        let begin_key = Included(iterator_test_bytes_user_key_of(2));
        let end_key = Excluded(iterator_test_bytes_user_key_of(7));

        let mut ui = UserIterator::for_test(mi, (begin_key, end_key));

        // ----- basic iterate -----
        ui.rewind().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- before-begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(1).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(2).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(7).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());

        // ----- after-end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(8).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());
    }

    // ..=right
    #[tokio::test]
    async fn test_range_to_inclusive() {
        let sstable_store = mock_sstable_store();
        // key=[idx, epoch], value

        let table = generate_test_data(sstable_store.clone(), vec![]).await;
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![SstableIterator::create(table, sstable_store, read_options)];
        let mi = MergeIterator::new(iters);
        let end_key = Included(iterator_test_bytes_user_key_of(7));

        let mut ui = UserIterator::for_test(mi, (Unbounded, end_key));

        // ----- basic iterate -----
        ui.rewind().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(1, 200));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(0).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(1, 200));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- in-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(2).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(7).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());

        // ----- after-end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(8).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());
    }

    // left..
    #[tokio::test]
    async fn test_range_from() {
        let sstable_store = mock_sstable_store();
        // key=[idx, epoch], value
        let table = generate_test_data(sstable_store.clone(), vec![]).await;
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let iters = vec![SstableIterator::create(table, sstable_store, read_options)];
        let mi = MergeIterator::new(iters);
        let begin_key = Included(iterator_test_bytes_user_key_of(2));

        let mut ui = UserIterator::for_test(mi, (begin_key, Unbounded));

        // ----- basic iterate -----
        ui.rewind().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(8, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(1).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(8, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- in-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(2).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(2, 300));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(3, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(6, 100));
        ui.next().await.unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(8, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(8).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key(), &iterator_test_bytes_key_of_epoch(8, 100));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- after-end-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(9).as_ref())
            .await
            .unwrap();
        assert!(!ui.is_valid());
    }

    #[tokio::test]
    async fn test_min_epoch() {
        let sstable_store = mock_sstable_store();
        let read_options = Arc::new(SstableIteratorReadOptions::default());
        let table0 = gen_iterator_test_sstable_with_incr_epoch(
            0,
            default_builder_opt_for_test(),
            |x| x * 3,
            sstable_store.clone(),
            TEST_KEYS_COUNT,
            1,
        )
        .await;
        let iters = vec![SstableIterator::create(
            table0,
            sstable_store.clone(),
            read_options.clone(),
        )];

        let min_epoch = (TEST_KEYS_COUNT / 5) as u64;
        let mi = MergeIterator::new(iters);
        let mut ui =
            UserIterator::for_test_with_epoch(mi, (Unbounded, Unbounded), u64::MAX, min_epoch);
        ui.rewind().await.unwrap();

        let mut i = 0;
        while ui.is_valid() {
            let key = ui.key();
            let key_epoch = key.epoch_with_gap.pure_epoch();
            assert!(key_epoch >= min_epoch);

            i += 1;
            ui.next().await.unwrap();
        }

        let expect_count = TEST_KEYS_COUNT - min_epoch as usize + 1;
        assert_eq!(i, expect_count);
    }

    #[tokio::test]
    async fn test_delete_range() {
        let sstable_store = mock_sstable_store();
        // key=[idx, epoch], value
        let table = generate_test_data(
            sstable_store.clone(),
            vec![(0, 2, 300), (1, 4, 150), (3, 6, 50), (5, 8, 150)],
        )
        .await;
        let read_options = SstableIteratorReadOptions::default();
        let iters = vec![SstableIterator::create(
            table.clone(),
            sstable_store.clone(),
            Arc::new(read_options),
        )];
        let mi = MergeIterator::new(iters);

        let mut del_iter = ForwardMergeRangeIterator::new(150);
        del_iter.add_sst_iter(SstableDeleteRangeIterator::new(table.clone()));
        let mut ui: UserIterator<_> =
            UserIterator::new(mi, (Unbounded, Unbounded), 150, 0, None, del_iter);

        // ----- basic iterate -----
        ui.rewind().await.unwrap();
        assert!(ui.is_valid());
        assert_eq!(ui.key().user_key, iterator_test_bytes_user_key_of(0));
        ui.next().await.unwrap();
        assert_eq!(ui.key().user_key, iterator_test_bytes_user_key_of(8));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        // ----- before-begin-range iterate -----
        ui.seek(iterator_test_bytes_user_key_of(1).as_ref())
            .await
            .unwrap();
        assert_eq!(ui.key().user_key, iterator_test_bytes_user_key_of(8));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());

        let read_options = SstableIteratorReadOptions::default();
        let iters = vec![SstableIterator::create(
            table.clone(),
            sstable_store,
            Arc::new(read_options),
        )];
        let mut del_iter = ForwardMergeRangeIterator::new(300);
        del_iter.add_sst_iter(SstableDeleteRangeIterator::new(table.clone()));
        let mi = MergeIterator::new(iters);
        let mut ui: UserIterator<_> =
            UserIterator::new(mi, (Unbounded, Unbounded), 300, 0, None, del_iter);
        ui.rewind().await.unwrap();
        assert!(ui.is_valid());
        assert_eq!(ui.key().user_key, iterator_test_bytes_user_key_of(2));
        ui.next().await.unwrap();
        assert_eq!(ui.key().user_key, iterator_test_bytes_user_key_of(8));
        ui.next().await.unwrap();
        assert!(!ui.is_valid());
    }
}
