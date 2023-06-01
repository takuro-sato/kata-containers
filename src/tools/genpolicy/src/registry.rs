// Copyright (c) 2023 Microsoft Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

// Allow Docker image config field names.
#![allow(non_snake_case)]

use crate::pod;
use crate::policy;

use anyhow::{anyhow, Result};
use log::{debug, info, LevelFilter};
use oci_distribution::client::{linux_amd64_resolver, ClientConfig};
use oci_distribution::{manifest, secrets::RegistryAuth, Client, Reference};
use serde::{Deserialize, Serialize};
use sha2::{digest::typenum::Unsigned, digest::OutputSizeUser, Sha256};
use std::{io, io::Seek, io::Write, path::Path};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct Container {
    config_layer: DockerConfigLayer,
    image_layers: Vec<ImageLayer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DockerConfigLayer {
    architecture: String,
    config: DockerImageConfig,
    rootfs: DockerRootfs,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DockerImageConfig {
    User: Option<String>,
    Tty: Option<bool>,
    Env: Vec<String>,
    Cmd: Option<Vec<String>>,
    WorkingDir: Option<String>,
    Entrypoint: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DockerRootfs {
    r#type: String,
    diff_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ImageLayer {
    pub diff_id: String,
    pub verity_hash: String,
}

impl Container {
    pub async fn new(use_cached_files: bool, image: &str) -> Result<Self> {
        info!("============================================");
        info!("Pulling manifest and config for {:?}", image);
        let reference: Reference = image.to_string().parse().unwrap();
        let mut client = Client::new(ClientConfig {
            platform_resolver: Some(Box::new(linux_amd64_resolver)),
            ..Default::default()
        });

        let (manifest, digest_hash, config_layer_str) = client
            .pull_manifest_and_config(&reference, &RegistryAuth::Anonymous)
            .await
            .unwrap();

        debug!("digest_hash: {:?}", digest_hash);
        debug!(
            "manifest: {}",
            serde_json::to_string_pretty(&manifest).unwrap()
        );

        // Log the contents of the config layer.
        if log::max_level() >= LevelFilter::Debug {
            let mut deserializer = serde_json::Deserializer::from_str(&config_layer_str);
            let mut serializer = serde_json::Serializer::pretty(io::stderr());
            serde_transcode::transcode(&mut deserializer, &mut serializer).unwrap();
        }

        let config_layer: DockerConfigLayer = serde_json::from_str(&config_layer_str).unwrap();
        let image_layers = get_image_layers(
            use_cached_files,
            &mut client,
            &reference,
            &manifest,
            &config_layer,
        )
        .await
        .unwrap();

        Ok(Container {
            config_layer,
            image_layers,
        })
    }

    // Convert Docker image config to policy data.
    pub fn get_process(
        &self,
        process: &mut policy::OciProcess,
        yaml_has_command: bool,
        yaml_has_args: bool,
    ) -> Result<()> {
        debug!("Getting process field from docker config layer...");
        let docker_config = &self.config_layer.config;

        if let Some(image_user) = &docker_config.User {
            if !image_user.is_empty() {
                debug!("Splitting Docker config user = {:?}", image_user);
                let user: Vec<&str> = image_user.split(':').collect();
                if !user.is_empty() {
                    debug!("Parsing user[0] = {:?}", user[0]);
                    process.user.uid = user[0].parse()?;
                    debug!("string: {:?} => uid: {}", user[0], process.user.uid);
                }
                if user.len() > 1 {
                    debug!("Parsing user[1] = {:?}", user[1]);
                    process.user.gid = user[1].parse()?;
                    debug!("string: {:?} => gid: {}", user[1], process.user.gid);
                }
            }
        }

        if let Some(terminal) = docker_config.Tty {
            process.terminal = terminal;
        } else {
            process.terminal = false;
        }

        for env in &docker_config.Env {
            process.env.push(env.clone());
        }

        let policy_args = &mut process.args;
        debug!("Already existing policy args: {:?}", policy_args);

        if let Some(entry_points) = &docker_config.Entrypoint {
            debug!("Image Entrypoint: {:?}", entry_points);
            if !yaml_has_command {
                debug!("Inserting Entrypoint into policy args");

                let mut reversed_entry_points = entry_points.clone();
                reversed_entry_points.reverse();

                for entry_point in reversed_entry_points {
                    policy_args.insert(0, entry_point.clone());
                }
            } else {
                debug!("Ignoring image Entrypoint because YAML specified the container command");
            }
        } else {
            debug!("No image Entrypoint");
        }

        debug!("Updated policy args: {:?}", policy_args);

        if yaml_has_command {
            debug!("Ignoring image Cmd because YAML specified the container command");
        } else if yaml_has_args {
            debug!("Ignoring image Cmd because YAML specified the container args");
        } else if let Some(commands) = &docker_config.Cmd {
            debug!("Adding to policy args the image Cmd: {:?}", commands);

            for cmd in commands {
                policy_args.push(cmd.clone());
            }
        } else {
            debug!("Image Cmd field is not present");
        }

        debug!("Updated policy args: {:?}", policy_args);

        if let Some(working_dir) = &docker_config.WorkingDir {
            if !working_dir.is_empty() {
                process.cwd = working_dir.clone();
            }
        }

        debug!("get_process succeeded.");
        Ok(())
    }

    pub fn get_image_layers(&self) -> Vec<ImageLayer> {
        self.image_layers.clone()
    }
}

async fn get_image_layers(
    use_cached_files: bool,
    client: &mut Client,
    reference: &Reference,
    manifest: &manifest::OciImageManifest,
    config_layer: &DockerConfigLayer,
) -> Result<Vec<ImageLayer>> {
    let mut layer_index = 0;
    let mut layers = Vec::new();

    for layer in &manifest.layers {
        if layer
            .media_type
            .eq(manifest::IMAGE_DOCKER_LAYER_GZIP_MEDIA_TYPE)
        {
            if layer_index < config_layer.rootfs.diff_ids.len() {
                layers.push(ImageLayer {
                    diff_id: config_layer.rootfs.diff_ids[layer_index].clone(),
                    verity_hash: get_verity_hash(
                        use_cached_files,
                        client,
                        reference,
                        &layer.digest,
                    )
                    .await?,
                });
            } else {
                return Err(anyhow!("Too many Docker gzip layers"));
            }

            layer_index += 1;
        }
    }

    Ok(layers)
}

fn delete_files(
    decompressed_path: &Path,
    compressed_path: &Path,
    verity_path: &Path,
) {
    let _ = fs::remove_file(&decompressed_path);
    let _ = fs::remove_file(&compressed_path);
    let _ = fs::remove_file(&verity_path);
}

async fn get_verity_hash(
    use_cached_files: bool,
    client: &mut Client,
    reference: &Reference,
    layer_digest: &str,
) -> Result<String> {
    let base_dir = std::path::Path::new("layers_cache");

    // Use file names supported by both Linux and Windows.
    let file_name = str::replace(&layer_digest, ":", "-");

    let mut decompressed_path = base_dir.join(file_name);
    decompressed_path.set_extension("tar");

    let mut compressed_path = decompressed_path.clone();
    compressed_path.set_extension("gz");

    let mut verity_path = decompressed_path.clone();
    verity_path.set_extension("verity");

    if use_cached_files && verity_path.exists() {
        info!("Using cached file {:?}", &verity_path);
    } else if let Err(e) = create_verity_hash_file(
            use_cached_files,
            client,
            reference,
            layer_digest,
            &base_dir,
            &decompressed_path,
            &compressed_path,
            &verity_path,
        ).await {
            delete_files(&decompressed_path, &compressed_path, &verity_path);
            panic!("Failed to create verity hash for {}, error {:?}", layer_digest, &e);
    }

    match std::fs::read_to_string(&verity_path) {
        Err(e) => {
            delete_files(&decompressed_path, &compressed_path, &verity_path);
            panic!("Failed to read {:?}, error {:?}", &verity_path, &e);
        },
        Ok(v) => {
            info!("dm-verity root hash: {}", &v);
            return Ok(v)
        }
    }
}

async fn create_verity_hash_file(
    use_cached_files: bool,
    client: &mut Client,
    reference: &Reference,
    layer_digest: &str,
    base_dir: &Path,
    decompressed_path: &Path,
    compressed_path: &Path,
    verity_path: &Path,
) -> Result<()> {
    if use_cached_files && decompressed_path.exists() {
        info!("Using cached file {:?}", &decompressed_path);
    } else {
        std::fs::create_dir_all(&base_dir)?;

        create_decompressed_layer_file(
            use_cached_files,
            client,
            reference,
            layer_digest,
            &decompressed_path,
            &compressed_path,
        ).await?;
    }

    do_create_verity_hash_file(decompressed_path, verity_path)
}

async fn create_decompressed_layer_file(
    use_cached_files: bool,
    client: &mut Client,
    reference: &Reference,
    layer_digest: &str,
    decompressed_path: &Path,
    compressed_path: &Path,
) -> Result<()> {
    if use_cached_files && compressed_path.exists() {
        info!("Using cached file {:?}", &compressed_path);
    } else {
        info!("Pulling layer {:?}", layer_digest);
        let mut file = tokio::fs::File::create(&compressed_path).await.map_err(|e| anyhow!(e))?;
        client.pull_blob(&reference, layer_digest, &mut file).await.map_err(|e| anyhow!(e))?;
        file.flush().await.map_err(|e| anyhow!(e))?;
    }

    info!("Decompressing layer");
    let compressed_file = std::fs::File::open(&compressed_path).map_err(|e| anyhow!(e))?;
    let mut decompressed_file = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&decompressed_path)?;
    let mut gz_decoder = flate2::read::GzDecoder::new(compressed_file);
    std::io::copy(&mut gz_decoder, &mut decompressed_file).map_err(|e| anyhow!(e))?;

    info!("Adding tarfs index to layer");
    decompressed_file.seek(std::io::SeekFrom::Start(0))?;
    tarindex::append_index(&mut decompressed_file).map_err(|e| anyhow!(e))?;
    decompressed_file.flush().map_err(|e| anyhow!(e))?;

    Ok(())
}

fn do_create_verity_hash_file(path: &Path, verity_path: &Path) -> Result<()> {
    info!("Calculating dm-verity root hash");
    let mut file = std::fs::File::open(path)?;
    let size = file.seek(std::io::SeekFrom::End(0))?;
    if size < 4096 {
        return Err(anyhow!("Block device {:?} is too small: {}", path, size));
    }

    let salt = [0u8; <Sha256 as OutputSizeUser>::OutputSize::USIZE];
    let v = verity::Verity::<Sha256>::new(size, 4096, 4096, &salt, 0)?;
    let hash = verity::traverse_file(&mut file, 0, false, v, &mut verity::no_write)?;
    let result = format!("{:x}", hash);

    let mut verity_file = std::fs::File::create(verity_path).map_err(|e| anyhow!(e))?;
    verity_file.write_all(result.as_bytes()).map_err(|e| anyhow!(e))?;
    verity_file.flush().map_err(|e| anyhow!(e))?;

    Ok(())
}

pub async fn get_registry_containers(
    use_cached_files: bool,
    yaml_containers: &Vec<pod::Container>,
) -> Result<Vec<Container>> {
    let mut registry_containers = Vec::new();

    for yaml_container in yaml_containers {
        registry_containers.push(Container::new(use_cached_files, &yaml_container.image).await?);
    }

    Ok(registry_containers)
}
