// Copyright (c) 2023 Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

// Allow K8s YAML field names.
#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emptyDir: Option<EmptyDirVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostPath: Option<HostPathVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistentVolumeClaim: Option<PersistentVolumeClaimVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub configMap: Option<ConfigMapVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub azureFile: Option<AzureFileVolumeSource>,
    // TODO: additional fields.
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostPathVolumeSource {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmptyDirVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizeLimit: Option<String>,
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistentVolumeClaimVolumeSource {
    pub claimName: String,
    // TODO: additional fields.
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigMapVolumeSource {
    pub name: String,
    pub items: Vec<KeyToPath>,
    // TODO: additional fields.
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyToPath {
    pub key: String,
    pub path: String,
}

/// See Reference / Kubernetes API / Config and Storage Resources / Volume.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AzureFileVolumeSource {
    pub secretName: String,
    pub shareName: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub readOnly: Option<bool>,
}
