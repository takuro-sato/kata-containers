// Copyright (c) 2023 Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use clap::Parser;
use env_logger;
use log::{debug, info};

mod config_map;
mod containerd;
mod daemon_set;
mod deployment;
mod infra;
mod job;
mod kata;
mod list;
mod no_policy;
mod obj_meta;
mod pause_container;
mod persistent_volume_claim;
mod pod;
mod pod_template;
mod policy;
mod registry;
mod replica_set;
mod replication_controller;
mod secret;
mod stateful_set;
mod utils;
mod volume;
mod yaml;

#[derive(Debug, Parser)]
struct CommandLineOptions {
    #[clap(short, long)]
    yaml_file: Option<String>,

    #[clap(short, long)]
    input_files_path: Option<String>,

    #[clap(short, long)]
    config_map_file: Option<String>,

    #[clap(short, long)]
    use_cached_files: bool,

    #[clap(short, long)]
    silent_unsupported_fields: bool,

    #[clap(short, long)]
    raw_out: bool,

    #[clap(short, long)]
    base64_out: bool,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = CommandLineOptions::parse();

    let mut config_map_files = Vec::new();
    if let Some(config_map_file) = &args.config_map_file {
        config_map_files.push(config_map_file.clone());
    }

    let config = utils::Config::new(
        args.use_cached_files,
        args.yaml_file,
        args.input_files_path,
        &config_map_files,
        args.silent_unsupported_fields,
        args.raw_out,
        args.base64_out,
    );

    debug!("Creating policy from yaml, infra data and rules files...");
    let mut policy = policy::AgentPolicy::from_files(&config)
        .await
        .unwrap();

    debug!("Exporting policy to yaml file...");
    policy.export_policy();
    info!("Success!");
}
