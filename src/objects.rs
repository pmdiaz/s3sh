use aws_sdk_s3::Client;
use anyhow::Result;
use tabled::{Table, Tabled};
use colored::*;
use std::path::Path;
use aws_sdk_s3::primitives::ByteStream;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Tabled)]
struct ObjectInfo {
    key: String,
    size: i64,
    last_modified: String,
}

pub async fn list_objects(client: &Client, bucket_name: &str) -> Result<()> {
    let resp = client.list_objects_v2().bucket(bucket_name).send().await?;
    
    let mut object_infos = Vec::new();

    for object in resp.contents() {
        let key = object.key().unwrap_or("<unknown>").to_string();
        let size = object.size().unwrap_or(0);
        let last_modified = object.last_modified()
            .map(|d| d.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        object_infos.push(ObjectInfo { key, size, last_modified });
    }

    if object_infos.is_empty() {
        println!("{}", "No objects found.".yellow());
    } else {
        let table = Table::new(object_infos).to_string();
        println!("{}", table);
    }

    Ok(())
}

pub async fn upload_object(client: &Client, bucket_name: &str, file_path: &str, key: Option<String>) -> Result<()> {
    let path = Path::new(file_path);
    let file_name = path.file_name().ok_or_else(|| anyhow::anyhow!("Invalid file path"))?.to_str().unwrap();
    let object_key = key.unwrap_or_else(|| file_name.to_string());

    let body = ByteStream::from_path(path).await?;
    let content_type = mime_guess::from_path(path).first_or_octet_stream();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} Uploading {msg}...")?
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"));
    pb.set_message(object_key.clone());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    client.put_object()
        .bucket(bucket_name)
        .key(&object_key)
        .body(body)
        .content_type(content_type.to_string())
        .send()
        .await?;

    pb.finish_with_message("Upload complete");
    println!("{} Object '{}' uploaded to '{}'.", "✔".green(), object_key, bucket_name);

    Ok(())
}

pub async fn delete_object(client: &Client, bucket_name: &str, key: &str) -> Result<()> {
    client.delete_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await?;

    println!("{} Object '{}' deleted from '{}'.", "✔".green(), key, bucket_name);
    Ok(())
}

pub async fn restore_object(client: &Client, bucket_name: &str, key: &str) -> Result<()> {
     client.restore_object()
        .bucket(bucket_name)
        .key(key)
        .restore_request(
            aws_sdk_s3::types::RestoreRequest::builder()
                .days(1)
                .glacier_job_parameters(
                    aws_sdk_s3::types::GlacierJobParameters::builder()
                        .tier(aws_sdk_s3::types::Tier::Standard)
                        .build()?
                )
                .build()
        )
        .send()
        .await?;

    println!("{} Restore request initiated for '{}'.", "✔".green(), key);
    Ok(())
}

pub async fn get_object_attributes(client: &Client, bucket_name: &str, key: &str) -> Result<()> {
    let resp = client.head_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await?;

    println!("Object: {}", key.bold());
    println!("Size: {} bytes", resp.content_length().unwrap_or(0));
    println!("Content Type: {}", resp.content_type().unwrap_or("unknown"));
    println!("Last Modified: {}", resp.last_modified().map(|d| d.to_string()).unwrap_or_else(|| "Unknown".to_string()));
    
    Ok(())
}
