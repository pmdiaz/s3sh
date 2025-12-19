use aws_sdk_s3::Client;
use anyhow::Result;
use tabled::{Table, Tabled};
use colored::*;
use aws_sdk_s3::error::ProvideErrorMetadata;
use aws_sdk_s3::types::LifecycleRuleFilter;

#[derive(Tabled)]
struct BucketInfo {
    name: String,
    creation_date: String,
}

pub async fn list_buckets(client: &Client) -> Result<()> {
    let resp = client.list_buckets().send().await?;
    let buckets = resp.buckets();

    let mut bucket_infos = Vec::new();

    for bucket in buckets {
        let name = bucket.name().unwrap_or("<unknown>").to_string();
        let creation_date = bucket.creation_date()
            .map(|d| d.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        bucket_infos.push(BucketInfo { name, creation_date });
    }

    if bucket_infos.is_empty() {
        println!("{}", "No buckets found.".yellow());
    } else {
        let table = Table::new(bucket_infos).to_string();
        println!("{}", table);
    }

    Ok(())
}

pub async fn create_bucket(
    client: &Client,
    bucket_name: &str,
    region: &str,
    public: Option<bool>,
    versioning: Option<bool>,
    encryption: Option<String>,
    tags: Vec<(String, String)>,
) -> Result<()> {
    validate_bucket_name(bucket_name)?;

    let mut builder = client.create_bucket().bucket(bucket_name);

    if region != "us-east-1" {
        let constraint = aws_sdk_s3::types::BucketLocationConstraint::from(region);
        let cfg = aws_sdk_s3::types::CreateBucketConfiguration::builder()
            .location_constraint(constraint)
            .build();
        builder = builder.create_bucket_configuration(cfg);
    }

    builder.send().await?;

    println!("{} Bucket '{}' created successfully.", "✔".green(), bucket_name);

    // Apply configurations if any
    if public.is_some() || versioning.is_some() || encryption.is_some() || !tags.is_empty() {
        println!("Applying configurations...");
        update_bucket(client, bucket_name, public, versioning, encryption, tags).await?;
    }

    Ok(())
}

pub fn validate_bucket_name(name: &str) -> Result<()> {
    if name.len() < 3 || name.len() > 63 {
        return Err(anyhow::anyhow!("Bucket name must be between 3 and 63 characters"));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_numeric() || c == '.' || c == '-') {
        return Err(anyhow::anyhow!("Bucket name must only contain lowercase letters, numbers, dots, and hyphens"));
    }
    if name.starts_with(|c| c == '.' || c == '-') || name.ends_with(|c| c == '.' || c == '-') {
         return Err(anyhow::anyhow!("Bucket name must begin and end with a letter or number"));
    }
    Ok(())
}

pub async fn get_bucket_config(client: &Client, bucket_name: &str) -> Result<()> {
    let location = client.get_bucket_location()
        .bucket(bucket_name)
        .send()
        .await?;
    
    let location_constraint = location.location_constraint().map(|l| l.as_str()).unwrap_or("us-east-1");
    
    println!("Bucket: {}", bucket_name.bold());
    println!("Region: {}", location_constraint.cyan());
    
    Ok(())
}

pub async fn update_bucket(
    client: &Client,
    bucket_name: &str,
    public: Option<bool>,
    versioning: Option<bool>,
    encryption: Option<String>,
    tags: Vec<(String, String)>,
) -> Result<()> {
    if let Some(is_public) = public {
        set_public_access(client, bucket_name, is_public).await?;
    }

    if let Some(enabled) = versioning {
        set_versioning(client, bucket_name, enabled).await?;
    }

    if let Some(mode) = encryption {
        set_encryption(client, bucket_name, &mode).await?;
    }

    if !tags.is_empty() {
        set_tags(client, bucket_name, tags).await?;
    }

    Ok(())
}

async fn set_public_access(client: &Client, bucket_name: &str, is_public: bool) -> Result<()> {
    let config = if is_public {
        aws_sdk_s3::types::PublicAccessBlockConfiguration::builder()
            .block_public_acls(false)
            .ignore_public_acls(false)
            .block_public_policy(false)
            .restrict_public_buckets(false)
            .build()
    } else {
        aws_sdk_s3::types::PublicAccessBlockConfiguration::builder()
            .block_public_acls(true)
            .ignore_public_acls(true)
            .block_public_policy(true)
            .restrict_public_buckets(true)
            .build()
    };

    client.put_public_access_block()
        .bucket(bucket_name)
        .public_access_block_configuration(config)
        .send()
        .await?;

    let status = if is_public { "Public" } else { "Private" };
    println!("{} Bucket '{}' public access set to: {}", "✔".green(), bucket_name, status.cyan());
    Ok(())
}

async fn set_versioning(client: &Client, bucket_name: &str, enabled: bool) -> Result<()> {
    let status = if enabled {
        aws_sdk_s3::types::BucketVersioningStatus::Enabled
    } else {
        aws_sdk_s3::types::BucketVersioningStatus::Suspended
    };

    let config = aws_sdk_s3::types::VersioningConfiguration::builder()
        .status(status.clone())
        .build();

    client.put_bucket_versioning()
        .bucket(bucket_name)
        .versioning_configuration(config)
        .send()
        .await?;

    println!("{} Bucket '{}' versioning set to: {}", "✔".green(), bucket_name, format!("{:?}", status).cyan());
    Ok(())
}

async fn set_encryption(client: &Client, bucket_name: &str, mode: &str) -> Result<()> {
    let rule = match mode {
        "AES256" => aws_sdk_s3::types::ServerSideEncryptionRule::builder()
            .apply_server_side_encryption_by_default(
                aws_sdk_s3::types::ServerSideEncryptionByDefault::builder()
                    .sse_algorithm(aws_sdk_s3::types::ServerSideEncryption::Aes256)
                    .build()?
            )
            .build(),
        "aws:kms" => aws_sdk_s3::types::ServerSideEncryptionRule::builder()
            .apply_server_side_encryption_by_default(
                aws_sdk_s3::types::ServerSideEncryptionByDefault::builder()
                    .sse_algorithm(aws_sdk_s3::types::ServerSideEncryption::AwsKms)
                    .build()?
            )
            .build(),
        _ => return Err(anyhow::anyhow!("Invalid encryption mode. Use 'AES256' or 'aws:kms'")),
    };

    let config = aws_sdk_s3::types::ServerSideEncryptionConfiguration::builder()
        .rules(rule)
        .build()?;

    client.put_bucket_encryption()
        .bucket(bucket_name)
        .server_side_encryption_configuration(config)
        .send()
        .await?;

    println!("{} Bucket '{}' encryption set to: {}", "✔".green(), bucket_name, mode.cyan());
    Ok(())
}

async fn set_tags(client: &Client, bucket_name: &str, tags: Vec<(String, String)>) -> Result<()> {
    let mut tag_set = Vec::new();
    for (k, v) in tags {
        tag_set.push(aws_sdk_s3::types::Tag::builder().key(k).value(v).build()?);
    }

    let tagging = aws_sdk_s3::types::Tagging::builder()
        .set_tag_set(Some(tag_set))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build tagging: {}", e))?;

    client.put_bucket_tagging()
        .bucket(bucket_name)
        .tagging(tagging)
        .send()
        .await?;

    println!("{} Bucket '{}' tags updated.", "✔".green(), bucket_name);
    Ok(())
}

#[derive(serde::Deserialize)]
struct TransitionInput {
    days: i32,
    storage_class: String,
}

pub async fn put_lifecycle_rule(
    client: &Client,
    bucket_name: &str,
    rule_id: &str,
    prefix: &str,
    transitions_json: &str,
    expiration_days: Option<i32>,
    status: bool,
) -> Result<()> {
    let transitions_input: Vec<TransitionInput> = serde_json::from_str(transitions_json)
        .map_err(|e| anyhow::anyhow!("Invalid transitions JSON: {}", e))?;

    let mut transitions = Vec::new();
    for t in transitions_input {
        let class = match t.storage_class.as_str() {
            "STANDARD_IA" => aws_sdk_s3::types::TransitionStorageClass::StandardIa,
            "ONEZONE_IA" => aws_sdk_s3::types::TransitionStorageClass::OnezoneIa,
            "INTELLIGENT_TIERING" => aws_sdk_s3::types::TransitionStorageClass::IntelligentTiering,
            "GLACIER" => aws_sdk_s3::types::TransitionStorageClass::Glacier,
            "DEEP_ARCHIVE" => aws_sdk_s3::types::TransitionStorageClass::DeepArchive,
             _ => return Err(anyhow::anyhow!("Invalid storage class: {}", t.storage_class)),
        };
        
        transitions.push(
            aws_sdk_s3::types::Transition::builder()
                .days(t.days)
                .storage_class(class)
                .build(),
        );
    }

    let expiration = if let Some(days) = expiration_days {
        Some(aws_sdk_s3::types::LifecycleExpiration::builder().days(days).build())
    } else {
        None
    };

    let rule_status = if status {
        aws_sdk_s3::types::ExpirationStatus::Enabled
    } else {
        aws_sdk_s3::types::ExpirationStatus::Disabled
    };

    let new_rule = aws_sdk_s3::types::LifecycleRule::builder()
        .id(rule_id)
        .filter(LifecycleRuleFilter::builder().prefix(prefix.to_string()).build())
        .status(rule_status)
        .set_transitions(Some(transitions))
        .set_expiration(expiration)
        .build()?;

    // Fetch existing config
    let current_config = client.get_bucket_lifecycle_configuration()
        .bucket(bucket_name)
        .send()
        .await;

    let mut rules = match current_config {
        Ok(output) => output.rules.unwrap_or_default(),
        Err(err) => {
            // Check if error is NoSuchLifecycleConfiguration
            if err.meta().code() == Some("NoSuchLifecycleConfiguration") {
                Vec::new()
            } else {
                 return Err(anyhow::anyhow!("Failed to get lifecycle config: {}", err));
            }
        }
    };

    // Remove existing rule with same ID
    rules.retain(|r| r.id.as_deref() != Some(rule_id));
    
    // Add new rule
    rules.push(new_rule);

    let lifecycle_config = aws_sdk_s3::types::BucketLifecycleConfiguration::builder()
        .set_rules(Some(rules))
        .build()?;

    client.put_bucket_lifecycle_configuration()
        .bucket(bucket_name)
        .lifecycle_configuration(lifecycle_config)
        .send()
        .await?;

    println!("{} Lifecycle rule '{}' set for bucket '{}'.", "✔".green(), rule_id, bucket_name);
    Ok(())
}


