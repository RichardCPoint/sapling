/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use std::{
    cmp::min,
    convert::TryInto,
    time::{Duration, Instant},
};

use anyhow::Error;

use futures::future::{FutureExt, TryFutureExt};
use futures_old::Future as OldFuture;
use rand::{thread_rng, Rng};
use stats::prelude::*;
use tokio::sync::watch;

// This can be tweaked later.
const MAX_LAG: Duration = Duration::from_secs(5);

define_stats! {
    prefix = "mononoke.sqlblob.lag_delay";
    total_delay_ms: dynamic_timeseries("{}.total_delay_ms", (entity: String); Rate, Sum),
    raw_lag_ms: dynamic_timeseries("{}.raw_lag_ms", (entity: String); Rate, Sum),
}

#[derive(Clone)]
pub struct BlobDelay {
    lag_receiver: watch::Receiver<Duration>,
    entity: Option<String>,
}

// Adds a small amount of random delay to desynchronise when waiting
async fn jitter_delay(raw_lag: Duration) {
    let delay =
        thread_rng().gen_range(Duration::from_secs(0), min(Duration::from_secs(1), raw_lag));
    tokio::time::delay_for(delay).await;
}

impl BlobDelay {
    pub fn dummy() -> Self {
        let (_, lag_receiver) = watch::channel(Duration::new(0, 0));
        Self {
            lag_receiver,
            entity: None,
        }
    }

    #[cfg(fbcode_build)]
    pub fn from_channel(lag_receiver: watch::Receiver<Duration>, name: String) -> Self {
        let entity = Some(name);
        Self {
            lag_receiver,
            entity,
        }
    }

    pub fn delay(&self) -> impl OldFuture<Item = (), Error = Error> {
        let mut lag_receiver = self.lag_receiver.clone();
        let entity = self.entity.clone();
        async move {
            let start_time = Instant::now();
            while let Some(raw_lag) = lag_receiver.recv().await {
                if raw_lag < MAX_LAG {
                    if start_time.elapsed() > Duration::from_secs(1) {
                        // No jittering for short delays, but jitter us about a bit if we've seen
                        // lag and waited for it to die down, so that next request is random
                        jitter_delay(raw_lag).await;
                    }
                    break;
                }
                if let Some(entity) = &entity {
                    let raw_lag_ms = raw_lag.as_millis().try_into();
                    if let Ok(raw_lag_ms) = raw_lag_ms {
                        STATS::raw_lag_ms.add_value(raw_lag_ms, (entity.clone(),))
                    }
                }
            }
            if let Some(entity) = &entity {
                let total_delay_ms = start_time.elapsed().as_millis().try_into();
                if let Ok(total_delay_ms) = total_delay_ms {
                    STATS::total_delay_ms.add_value(total_delay_ms, (entity.clone(),));
                }
            }
            Ok(())
        }
        .boxed()
        .compat()
    }
}
