// Copyright (c) 2023 Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

// Allow K8s YAML field names.
#![allow(non_snake_case)]

use crate::obj_meta;
use crate::pod;
use crate::pod_template;
use crate::policy;
use crate::registry;
use crate::yaml;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// See Reference Kubernetes API / Workload Resources / DaemonSet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaemonSet {
    apiVersion: String,
    kind: String,
    pub metadata: obj_meta::ObjectMeta,
    pub spec: DaemonSetSpec,

    #[serde(skip)]
    doc_mapping: serde_yaml::Value,

    #[serde(skip)]
    registry_containers: Vec<registry::Container>,
}

/// See Reference Kubernetes API / Workload Resources / DaemonSet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaemonSetSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<yaml::LabelSelector>,

    pub template: pod_template::PodTemplateSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    minReadySeconds: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    updateStrategy: Option<DaemonSetUpdateStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    revisionHistoryLimit: Option<i32>,
}

/// See Reference Kubernetes API / Workload Resources / DaemonSet.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DaemonSetUpdateStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    rollingUpdate: Option<RollingUpdateDaemonSet>,
}

/// See Reference Kubernetes API / Workload Resources / DaemonSet.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RollingUpdateDaemonSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    maxSurge: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    maxUnavailable: Option<i32>,
}

#[async_trait]
impl yaml::K8sResource for DaemonSet {
    async fn init(
        &mut self,
        use_cache: bool,
        doc_mapping: &serde_yaml::Value,
        _silent_unsupported_fields: bool,
    ) -> anyhow::Result<()> {
        yaml::k8s_resource_init(
            &mut self.spec.template.spec,
            &mut self.registry_containers,
            use_cache,
        )
        .await?;
        self.doc_mapping = doc_mapping.clone();
        Ok(())
    }

    fn get_metadata_name(&self) -> String {
        self.metadata.get_name()
    }

    fn get_host_name(&self) -> String {
        "^".to_string() + &self.get_metadata_name() + "-[a-z0-9]{5}$"
    }

    fn get_sandbox_name(&self) -> Option<String> {
        None
    }

    fn get_namespace(&self) -> String {
        self.metadata.get_namespace()
    }

    fn get_pod_spec(&self) -> Option<&PodSpec> {
        Some(&self.spec.template.spec)
    }

    fn generate_policy(&self, agent_policy: &policy::AgentPolicy) -> String {
        yaml::generate_policy(self, agent_policy)
    }

    fn serialize(&mut self, policy: &str) -> String {
        yaml::add_policy_annotation(&mut self.doc_mapping, "spec.template.metadata", policy);
        serde_yaml::to_string(&self.doc_mapping).unwrap()
    }

    fn get_containers(&self) -> (&Vec<registry::Container>, &Vec<pod::Container>) {
        (
            &self.registry_containers,
            &self.spec.template.spec.containers,
        )
    }
}
