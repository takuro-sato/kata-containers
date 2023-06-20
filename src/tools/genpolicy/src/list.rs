// Copyright (c) 2023 Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

// Allow K8s YAML field names.
#![allow(non_snake_case)]

use crate::pod;
use crate::policy;
use crate::registry;
use crate::yaml;

use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::boxed;
use std::marker::{Send, Sync};

#[derive(Debug, Serialize, Deserialize)]
pub struct List {
    apiVersion: String,
    kind: String,

    items: Vec<serde_yaml::Value>,

    #[serde(skip)]
    resources: Vec<boxed::Box<dyn yaml::K8sResource + Sync + Send>>,
}

impl Debug for dyn yaml::K8sResource + Send + Sync {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "K8sResource")
    }
}

#[async_trait]
impl yaml::K8sResource for List {
    async fn init(
        &mut self,
        use_cache: bool,
        _doc_mapping: &serde_yaml::Value,
        silent_unsupported_fields: bool,
    ) -> anyhow::Result<()> {
        for item in &self.items {
            let yaml_string = serde_yaml::to_string(&item)?;
            let (mut resource, _kind) =
                yaml::new_k8s_resource(&yaml_string, silent_unsupported_fields)?;
            resource
                .init(use_cache, item, silent_unsupported_fields)
                .await?;
            self.resources.push(resource);
        }

        Ok(())
    }

    fn get_metadata_name(&self) -> String {
        panic!("Unsupported");
    }

    fn get_host_name(&self) -> String {
        panic!("Unsupported");
    }

    fn get_sandbox_name(&self) -> Option<String> {
        panic!("Unsupported");
    }

    fn get_namespace(&self) -> String {
        panic!("Unsupported");
    }

    fn get_pod_spec(&self) -> Option<&pod::PodSpec> {
        None
    }

    fn generate_policy(&self, agent_policy: &policy::AgentPolicy) -> String {
        let mut policies: Vec<String> = Vec::new();
        for resource in &self.resources {
            policies.push(resource.generate_policy(agent_policy));
        }
        policies.join(":")
    }

    fn serialize(&mut self, policy: &str) -> String {
        let policies: Vec<&str> = policy.split(":").collect();
        let len = policies.len();
        assert!(len == self.resources.len());

        self.items.clear();
        for i in 0..len {
            let yaml = self.resources[i].serialize(policies[i]);
            let document = serde_yaml::Deserializer::from_str(&yaml);
            let doc_value = Value::deserialize(document).unwrap();
            self.items.push(doc_value.clone());
        }
        serde_yaml::to_string(&self).unwrap()
    }

    fn get_containers(&self) -> (&Vec<registry::Container>, &Vec<pod::Container>) {
        panic!("Unsupported");
    }
}
