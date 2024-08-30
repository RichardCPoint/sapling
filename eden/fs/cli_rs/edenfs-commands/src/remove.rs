/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! edenfsctl remove

use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

use crate::ExitCode;
use crate::Subcommand;

#[derive(Parser, Debug)]
#[clap(name = "remove", about = "Remove an EdenFS checkout")]
pub struct RemoveCmd {
    #[clap(multiple_values = true, help = "The EdenFS checkout(s) to remove.")]
    path: Vec<String>,

    #[clap(
            short = 'y',
            visible_aliases = &["--yes", "--no-prompt"],
            help = "Do not prompt for confirmation before removing the checkouts."
        )]
    prompt: bool,

    #[clap(long, hide = true)]
    preserve_mount_point: bool,
}

#[async_trait]
impl Subcommand for RemoveCmd {
    async fn run(&self) -> Result<ExitCode> {
        Err(anyhow!("Rust remove is unimplemented!"))
    }
}
