// Copyright (c) Microsoft Corporation.
// Licensed under the Apache 2.0 license.

use anyhow::{bail, Result};
use checked_command::{CheckedCommand, Error};
use oci_spec::runtime::{Process, Spec};
use std::path::PathBuf;

const KUBECTL: &str = "kubectl";

// The default image version of the pause container is based
// on https://github.com/kubernetes/kubernetes/blob/release-1.23/cmd/kubeadm/app/constants/constants.go#L415
// The Kubernetes version (currently 1.23) is based on
// https://github.com/kata-containers/kata-containers/blob/CCv0/versions.yaml#L243
pub const KUBERNETES_PAUSE_VERSION: &str = "3.6";
pub const KUBERNETES_PAUSE_NAME: &str = "pause";
pub const KUBERNETES_REGISTRY: &str = "registry.k8s.io";

pub struct KubeCtl;

impl KubeCtl {
    pub fn get_config_map(name: &str) -> Result<serde_yaml::Value> {
        let output = match CheckedCommand::new(KUBECTL)
            .arg("get")
            .arg("configmap")
            .arg(name)
            .arg("-o")
            .arg("yaml")
            .output()
        {
            Ok(result) => String::from_utf8(result.stdout)?,
            Err(Error::Failure(ex, output)) => {
                println!("failed with exit code: {:?}", ex.code());
                if let Some(output) = output {
                    bail!(
                        "{}: kubectl failed: {}",
                        loc!(),
                        String::from_utf8_lossy(&*output.stderr)
                    );
                }
                bail!("{}", loc!());
            }
            Err(Error::Io(io_err)) => {
                bail!("{}: unexpected I/O error: {:?}", loc!(), io_err);
            }
        };

        let config_map: serde_yaml::Value = serde_yaml::from_str(&output)?;

        Ok(config_map)
    }

    pub fn get_yaml_with_dry_run_server(file: &PathBuf) -> Result<serde_yaml::Value> {
        let output = match CheckedCommand::new(KUBECTL)
            .arg("apply")
            .arg("-f")
            .arg(file)
            .arg("--dry-run=server")
            .arg("-o")
            .arg("yaml")
            .output()
        {
            Ok(result) => String::from_utf8(result.stdout)?,
            Err(Error::Failure(ex, output)) => {
                println!("failed with exit code: {:?}", ex.code());
                if let Some(output) = output {
                    bail!(
                        "{}: kubectl failed: {}",
                        loc!(),
                        String::from_utf8_lossy(&*output.stderr)
                    );
                }
                bail!("{}", loc!());
            }
            Err(Error::Io(io_err)) => {
                bail!("{}: unexpected I/O error: {:?}", loc!(), io_err);
            }
        };

        let pod_yaml: serde_yaml::Value = serde_yaml::from_str(&output)?;

        Ok(pod_yaml)
    }
}

fn get_container_rules() -> Result<Spec> {
    let mut spec: Spec = serde_json::from_str("{}")?;

    // Initialize with necessary fields
    let mut process: Process = serde_json::from_str(
        r#"{
        "user": {
            "uid": 0,
            "gid": 0   
        },
        "cwd": ""
    }"#,
    )?;

    // Add environment variables that allow the container to find services
    // Reference: https://github.com/kubernetes/kubernetes/blob/release-1.26/pkg/kubelet/envvars/envvars.go#L32
    let env = [
        "^[A-Z0-9_]+_SERVICE_HOST=^((25[0-5]|(2[0-4]|1\\d|[1-9]|)\\d).?\\b){4}$",
        "^[A-Z0-9_]+_SERVICE_PORT=[0-9]+",
        "^[A-Z0-9_]+_SERVICE_PORT_[A-Z]+=[0-9]+",
        "^[A-Z0-9_]+_PORT=[a-z]+://^((25[0-5]|(2[0-4]|1\\d|[1-9]|)\\d).?\\b){4}:[0-9]+",
        "^[A-Z0-9_]+_PORT_[0-9]+_[A-Z]+=[a-z]+://^((25[0-5]|(2[0-4]|1\\d|[1-9]|)\\d).?\\b){4}:[0-9]+",
        "^[A-Z0-9_]+_PORT_[0-9]+_[A-Z]+_PROTO=[a-z]+",
        "^[A-Z0-9_]+_PORT_[0-9]+_[A-Z]+_PORT=[0-9]+",
        "^[A-Z0-9_]+_PORT_[0-9]+_[A-Z]+_ADDR=^((25[0-5]|(2[0-4]|1\\d|[1-9]|)\\d).?\\b){4}$" 
    ].map(String::from).to_vec();

    process.set_env(Some(env));

    spec.set_process(Some(process));

    Ok(spec)
}

// TODO: Check if there is any sandbox-specific insertions
fn get_sandbox_rules() -> Result<Spec> {
    let spec = serde_json::from_str("{}")?;

    Ok(spec)
}

pub fn get_rules(is_sandbox: bool) -> Result<Spec> {
    if !is_sandbox {
        get_container_rules()
    } else {
        get_sandbox_rules()
    }
}

pub fn get_pause_image_ref() -> String {
    [
        KUBERNETES_REGISTRY,
        "/",
        KUBERNETES_PAUSE_NAME,
        ":",
        KUBERNETES_PAUSE_VERSION,
    ]
    .concat()
}
