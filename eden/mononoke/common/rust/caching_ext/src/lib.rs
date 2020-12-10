/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#![deny(warnings)]
#![feature(never_type)]

mod cachelib_utils;
mod memcache_utils;
mod mock_store;

use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::time::Duration;

use abomonation::Abomonation;
use anyhow::{Context as _, Error};
use async_trait::async_trait;
use auto_impl::auto_impl;
use bytes::Bytes;
use cloned::cloned;
use futures::future;
use memcache::{KeyGen, MEMCACHE_VALUE_MAX_SIZE};

pub use crate::cachelib_utils::CachelibHandler;
pub use crate::memcache_utils::MemcacheHandler;
pub use crate::mock_store::MockStoreStats;

/// Error type to help with proper reporting of memcache errors
pub enum McErrorKind {
    /// error came from calling memcache API
    MemcacheInternal,
    /// value returned from memcache was None
    Missing,
    /// deserialization of memcache data to Rust structures failed
    Deserialization,
}

pub type McResult<T> = Result<T, McErrorKind>;

struct CachelibKey(String);
struct MemcacheKey(String);

#[derive(Copy, Clone)]
pub enum CacheTtl {
    NoTtl,
    Ttl(Duration),
}

#[derive(Copy, Clone)]
pub enum CacheDisposition {
    Cache(CacheTtl),
    Ignore,
}

pub trait MemcacheEntity: Sized {
    fn serialize(&self) -> Bytes;

    fn deserialize(bytes: Bytes) -> Result<Self, ()>;

    fn report_mc_result(res: &McResult<Self>); // TODO: Default impl here
}

#[auto_impl(&)]
pub trait EntityStore<V> {
    fn cachelib(&self) -> &CachelibHandler<V>;

    fn keygen(&self) -> &KeyGen;

    fn memcache(&self) -> &MemcacheHandler;

    fn cache_determinator(&self, v: &V) -> CacheDisposition;

    /// Whether Memcache writes should run in the background. This is normally the desired behavior
    /// so this defaults to true, but for tests it's useful to run them synchronously to get
    /// consistent outcomes.
    fn spawn_memcache_writes(&self) -> bool {
        true
    }
}

#[async_trait]
#[auto_impl(&)]
pub trait KeyedEntityStore<K, V>: EntityStore<V> {
    fn get_cache_key(&self, key: &K) -> String;

    async fn get_from_db(&self, keys: HashSet<K>) -> Result<HashMap<K, V>, Error>;
}

pub async fn get_or_fill<K, V>(
    store: impl KeyedEntityStore<K, V>,
    keys: HashSet<K>,
) -> Result<HashMap<K, V>, Error>
where
    K: Hash + Eq + Clone,
    // TODO: We should relax the bounds on cachelib's set_cached. We don't need all of this:
    V: Abomonation + MemcacheEntity + Send + Clone + 'static,
{
    let mut ret = HashMap::<K, V>::new();

    let cachelib_keys: Vec<_> = keys
        .into_iter()
        .map(|key| {
            let cachelib_key = CachelibKey(store.get_cache_key(&key));
            (key, cachelib_key)
        })
        .collect();

    let (fetched_from_cachelib, to_fetch_from_memcache) = store
        .cachelib()
        .get_multiple_from_cachelib::<K>(cachelib_keys)
        .with_context(|| "Error reading from cachelib")?;

    ret.extend(fetched_from_cachelib);

    let to_fetch_from_memcache: Vec<(K, CachelibKey, MemcacheKey)> = to_fetch_from_memcache
        .into_iter()
        .map(|(key, cachelib_key)| {
            let memcache_key = MemcacheKey(store.keygen().key(&cachelib_key.0));
            (key, cachelib_key, memcache_key)
        })
        .collect();

    let to_fetch_from_store = {
        let (fetched_from_memcache, to_fetch_from_store) =
            get_multiple_from_memcache(store.memcache(), to_fetch_from_memcache).await;

        fill_multiple_cachelib(
            store.cachelib(),
            fetched_from_memcache
                .values()
                .filter_map(|(v, k)| match store.cache_determinator(v) {
                    CacheDisposition::Cache(ttl) => Some((k, ttl, v)),
                    _ => None,
                }),
        );

        ret.extend(fetched_from_memcache.into_iter().map(|(k, (v, _))| (k, v)));

        to_fetch_from_store
    };

    let mut key_mapping = HashMap::new();
    let to_fetch_from_store: HashSet<K> = to_fetch_from_store
        .into_iter()
        .map(|(key, cachelib_key, memcache_key)| {
            key_mapping.insert(key.clone(), (cachelib_key, memcache_key));
            key
        })
        .collect();

    if !to_fetch_from_store.is_empty() {
        let data = store
            .get_from_db(to_fetch_from_store)
            .await
            .with_context(|| "Error reading from store")?;

        fill_caches_by_key(
            store,
            data.iter().map(|(key, v)| {
                let (cachelib_key, memcache_key) = key_mapping
                    .remove(&key)
                    .expect("caching_ext: Missing entry in key_mapping, this should not happen");

                (cachelib_key, memcache_key, v)
            }),
        )
        .await;

        ret.extend(data);
    };

    Ok(ret)
}

pub async fn fill_cache<'a, K, V>(
    store: impl KeyedEntityStore<K, V>,
    data: impl IntoIterator<Item = (&'a K, &'a V)>,
) where
    K: Hash + Eq + Clone + 'a,
    V: Abomonation + MemcacheEntity + Send + Clone + 'static,
{
    fill_caches_by_key(
        &store,
        data.into_iter().map(|(k, v)| {
            let cachelib_key = CachelibKey(store.get_cache_key(&k));
            let memcache_key = MemcacheKey(store.keygen().key(&cachelib_key.0));
            (cachelib_key, memcache_key, v)
        }),
    )
    .await;
}

async fn fill_caches_by_key<'a, V>(
    store: impl EntityStore<V>,
    data: impl IntoIterator<Item = (CachelibKey, MemcacheKey, &'a V)>,
) where
    V: Abomonation + MemcacheEntity + Send + Clone + 'static,
{
    let mut cachelib_keys = Vec::new();
    let mut memcache_keys = Vec::new();

    for (cachelib_key, memcache_key, v) in data.into_iter() {
        let ttl = match store.cache_determinator(v) {
            CacheDisposition::Cache(ttl) => ttl,
            CacheDisposition::Ignore => continue,
        };

        memcache_keys.push((memcache_key, ttl, v));
        cachelib_keys.push((cachelib_key, ttl, v));
    }

    fill_multiple_cachelib(store.cachelib(), cachelib_keys);

    fill_multiple_memcache(
        store.memcache(),
        memcache_keys,
        store.spawn_memcache_writes(),
    )
    .await;
}

async fn get_multiple_from_memcache<K, V>(
    memcache: &MemcacheHandler,
    keys: Vec<(K, CachelibKey, MemcacheKey)>,
) -> (
    HashMap<K, (V, CachelibKey)>,
    Vec<(K, CachelibKey, MemcacheKey)>,
)
where
    K: Eq + Hash,
    V: MemcacheEntity,
{
    let mc_fetch_futs: Vec<_> = keys
        .into_iter()
        .map(move |(key, cachelib_key, memcache_key)| {
            cloned!(memcache);
            async move {
                let res = memcache
                    .get(memcache_key.0.clone())
                    .await
                    .map_err(|()| McErrorKind::MemcacheInternal)
                    .and_then(|maybe_bytes| maybe_bytes.ok_or(McErrorKind::Missing))
                    .and_then(|bytes| {
                        V::deserialize(bytes).map_err(|()| McErrorKind::Deserialization)
                    });

                (key, cachelib_key, memcache_key, res)
            }
        })
        .collect();

    let entries = future::join_all(mc_fetch_futs).await;

    let mut fetched = HashMap::new();
    let mut left_to_fetch = Vec::new();

    for (key, cachelib_key, memcache_key, res) in entries {
        V::report_mc_result(&res);

        match res {
            Ok(entity) => {
                fetched.insert(key, (entity, cachelib_key));
            }
            Err(..) => {
                left_to_fetch.push((key, cachelib_key, memcache_key));
            }
        }
    }

    (fetched, left_to_fetch)
}

fn fill_multiple_cachelib<'a, V>(
    cachelib: &'a CachelibHandler<V>,
    data: impl IntoIterator<Item = (impl Borrow<CachelibKey> + 'a, CacheTtl, &'a V)>,
) where
    V: Abomonation + Clone + Send + 'static,
{
    for (cachelib_key, ttl, v) in data {
        let cachelib_key = cachelib_key.borrow();

        match ttl {
            CacheTtl::NoTtl => {
                // NOTE: We ignore failures to cache individual entries here.
                let _ = cachelib.set_cached(&cachelib_key.0, v);
            }
            CacheTtl::Ttl(..) => {
                // Not implemented yet for our cachelib cache.
            }
        }
    }
}

async fn fill_multiple_memcache<'a, V: 'a>(
    memcache: &'a MemcacheHandler,
    data: impl IntoIterator<Item = (MemcacheKey, CacheTtl, &'a V)>,
    spawn: bool,
) where
    V: MemcacheEntity,
{
    let futs = data.into_iter().filter_map(|(memcache_key, ttl, v)| {
        let bytes = v.serialize();

        if bytes.len() >= MEMCACHE_VALUE_MAX_SIZE {
            return None;
        }

        cloned!(memcache);

        Some(async move {
            match ttl {
                CacheTtl::NoTtl => {
                    memcache.set(memcache_key.0, bytes).await?;
                }
                CacheTtl::Ttl(ttl) => {
                    memcache.set_with_ttl(memcache_key.0, bytes, ttl).await?;
                }
            }

            Result::<_, ()>::Ok(())
        })
    });

    let fut = future::join_all(futs);

    if spawn {
        tokio::task::spawn(fut);
    } else {
        fut.await;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use abomonation_derive::Abomonation;
    use maplit::{hashmap, hashset};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Abomonation, Clone, Debug, PartialEq, Eq)]
    struct TestEntity(Vec<u8>);

    impl MemcacheEntity for TestEntity {
        fn serialize(&self) -> Bytes {
            Bytes::from(self.0.clone())
        }

        fn deserialize(bytes: Bytes) -> Result<Self, ()> {
            Ok(Self(bytes.to_vec()))
        }

        fn report_mc_result(_: &McResult<Self>) {}
    }

    struct TestStore {
        keygen: KeyGen,
        cachelib: CachelibHandler<TestEntity>,
        memcache: MemcacheHandler,
        calls: AtomicUsize,
        keys: AtomicUsize,
        data: HashMap<String, TestEntity>,
    }

    impl TestStore {
        pub fn new() -> Self {
            Self {
                keygen: KeyGen::new("", 0, 0),
                cachelib: CachelibHandler::create_mock(),
                memcache: MemcacheHandler::create_mock(),
                calls: AtomicUsize::new(0),
                keys: AtomicUsize::new(0),
                data: HashMap::new(),
            }
        }
    }

    impl EntityStore<TestEntity> for TestStore {
        fn cachelib(&self) -> &CachelibHandler<TestEntity> {
            &self.cachelib
        }

        fn keygen(&self) -> &KeyGen {
            &self.keygen
        }

        fn memcache(&self) -> &MemcacheHandler {
            &self.memcache
        }

        fn cache_determinator(&self, _: &TestEntity) -> CacheDisposition {
            CacheDisposition::Cache(CacheTtl::NoTtl)
        }

        fn spawn_memcache_writes(&self) -> bool {
            false
        }
    }

    #[async_trait]
    impl KeyedEntityStore<String, TestEntity> for TestStore {
        fn get_cache_key(&self, key: &String) -> String {
            format!("key:{}", key)
        }

        async fn get_from_db(
            &self,
            keys: HashSet<String>,
        ) -> Result<HashMap<String, TestEntity>, Error> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.keys.fetch_add(keys.len(), Ordering::Relaxed);

            Ok(keys
                .into_iter()
                .filter_map(|k| {
                    let v = self.data.get(&k).cloned();
                    v.map(|v| (k, v))
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn simple() -> Result<(), Error> {
        let store = TestStore::new();

        let res = get_or_fill(&store, hashset! {}).await?;
        assert_eq!(res.len(), 0);
        assert_eq!(store.cachelib.gets_count(), 0);
        assert_eq!(store.memcache.gets_count(), 0);

        let res = get_or_fill(&store, hashset! {"key".into()}).await?;
        assert_eq!(res.len(), 0);
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 1);
        assert_eq!(store.keys.load(Ordering::Relaxed), 1);

        Ok(())
    }

    #[tokio::test]
    async fn fetch_from_db_cachelib_memcache() -> Result<(), Error> {
        let mut store = TestStore::new();

        let e = TestEntity(vec![0]);
        store.data.insert("key".into(), e.clone());

        // Fetch from db
        let res = get_or_fill(&store, hashset! {"key".into()}).await?;
        assert_eq!(res, hashmap! { "key".into() => e.clone() });
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 1);
        assert_eq!(store.keys.load(Ordering::Relaxed), 1);

        // Now fetch from cachelib
        let res = get_or_fill(&store, hashset! {"key".into()}).await?;
        assert_eq!(res, hashmap! { "key".into() => e.clone() });
        assert_eq!(store.cachelib.gets_count(), 2);
        assert_eq!(store.memcache.gets_count(), 1);
        assert_eq!(store.keys.load(Ordering::Relaxed), 1);

        // Reset cachelib, fetch from memcache
        store.cachelib = CachelibHandler::create_mock();
        let res = get_or_fill(&store, hashset! {"key".into()}).await?;
        assert_eq!(res, hashmap! { "key".into() => e.clone() });
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 2);
        assert_eq!(store.keys.load(Ordering::Relaxed), 1);

        Ok(())
    }

    #[tokio::test]
    async fn fetch_from_db() -> Result<(), Error> {
        let mut store = TestStore::new();

        let e0 = TestEntity(vec![0]);
        let e1 = TestEntity(vec![1]);
        let e2 = TestEntity(vec![2]);

        store.data.insert("key0".into(), e0.clone());
        store.data.insert("key1".into(), e1.clone());
        store.data.insert("key2".into(), e2.clone());

        let res = get_or_fill(
            &store,
            hashset! { "key0".into(), "key1".into(), "key2".into() },
        )
        .await?;

        assert_eq!(
            res,
            hashmap! { "key0".into() => e0, "key1".into() => e1, "key2".into() => e2 }
        );
        assert_eq!(store.cachelib.gets_count(), 3);
        assert_eq!(store.memcache.gets_count(), 3);
        assert_eq!(store.keys.load(Ordering::Relaxed), 3);

        Ok(())
    }

    #[tokio::test]
    async fn fetch_from_all() -> Result<(), Error> {
        let mut store = TestStore::new();

        let e0 = TestEntity(vec![0]);
        let e1 = TestEntity(vec![1]);
        let e2 = TestEntity(vec![2]);

        store.data.insert("key0".into(), e0.clone());
        store.data.insert("key1".into(), e1.clone());
        store.data.insert("key2".into(), e2.clone());

        let res = get_or_fill(&store, hashset! { "key1".into() }).await?;
        assert_eq!(res, hashmap! { "key1".into() => e1.clone() });
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 1);
        assert_eq!(store.calls.load(Ordering::Relaxed), 1);

        // Reset cachelib
        store.cachelib = CachelibHandler::create_mock();
        let res = get_or_fill(&store, hashset! { "key0".into() }).await?;
        assert_eq!(res, hashmap! { "key0".into() => e0.clone() });
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 2);
        assert_eq!(store.calls.load(Ordering::Relaxed), 2);

        let res = get_or_fill(
            &store,
            hashset! { "key0".into(), "key1".into(), "key2".into() },
        )
        .await?;

        assert_eq!(
            res,
            hashmap! { "key0".into() => e0.clone(), "key1".into() => e1.clone(), "key2".into() => e2.clone() }
        );
        assert_eq!(store.cachelib.gets_count(), 1 + 3); // 3 new fetches from cachelib, 2 misses
        assert_eq!(store.memcache.gets_count(), 2 + 2); // 2 new fetches from memcache, 1 miss
        assert_eq!(store.calls.load(Ordering::Relaxed), 2 + 1); // 1 fetch from db

        // Only from cachelib
        let res = get_or_fill(
            &store,
            hashset! { "key0".into(), "key1".into(), "key2".into() },
        )
        .await?;

        assert_eq!(
            res,
            hashmap! { "key0".into() => e0.clone(), "key1".into() => e1.clone(), "key2".into() => e2.clone() }
        );
        assert_eq!(store.cachelib.gets_count(), 7);
        assert_eq!(store.memcache.gets_count(), 4);
        assert_eq!(store.calls.load(Ordering::Relaxed), 3);

        // // Reset cachelib, only from memcache
        store.cachelib = CachelibHandler::create_mock();
        let res = get_or_fill(
            &store,
            hashset! { "key0".into(), "key1".into(), "key2".into() },
        )
        .await?;

        assert_eq!(
            res,
            hashmap! { "key0".into() => e0.clone(), "key1".into() => e1.clone(), "key2".into() => e2.clone() }
        );
        assert_eq!(store.cachelib.gets_count(), 3); // 3 misses
        assert_eq!(store.memcache.gets_count(), 4 + 3); // 3 hits
        assert_eq!(store.calls.load(Ordering::Relaxed), 3);

        Ok(())
    }

    #[tokio::test]
    async fn get_from_db_elision() -> Result<(), Error> {
        let store = TestStore::new();

        get_or_fill(&store, hashset! {}).await?;
        assert_eq!(store.calls.load(Ordering::Relaxed), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_fill_cache() -> Result<(), Error> {
        let store = TestStore::new();
        let e0 = TestEntity(vec![0]);
        fill_cache(&store, hashmap! { "key0".into() => e0.clone() }.iter()).await;

        let res = get_or_fill(&store, hashset! { "key0".into() }).await?;
        assert_eq!(res, hashmap! { "key0".into() => e0.clone() });
        assert_eq!(store.cachelib.gets_count(), 1);
        assert_eq!(store.memcache.gets_count(), 0);
        assert_eq!(store.calls.load(Ordering::Relaxed), 0);

        Ok(())
    }
}
