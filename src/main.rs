use clap::{Parser, Subcommand};
use anyhow::Result;
use s3sh::{client, buckets, objects};

#[derive(Parser)]
#[command(name = "s3sh")]
#[command(about = "A simple S3 CLI in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// AWS Region
    #[arg(short, long, global = true)]
    region: Option<String>,

    /// AWS Profile
    #[arg(short, long, global = true)]
    profile: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage Buckets
    Bucket {
        #[command(subcommand)]
        action: BucketAction,
    },
    /// Manage Objects
    Object {
        #[command(subcommand)]
        action: ObjectAction,
    },
}

#[derive(Subcommand)]
enum BucketAction {
    /// List all buckets
    List,
    /// Create a new bucket
    Create {
        /// Name of the bucket
        name: String,

        /// Set public access (true = public, false = private/block all)
        #[arg(long)]
        public: Option<bool>,

        /// Enable or disable versioning
        #[arg(long)]
        versioning: Option<bool>,

        /// Set encryption mode (AES256 or aws:kms)
        #[arg(long)]
        encryption: Option<String>,

        /// Add tags (Key=Value)
        #[arg(long, value_parser = parse_key_val::<String, String>)]
        tags: Vec<(String, String)>,
    },
    /// Get bucket configuration
    Config {
        /// Name of the bucket
        name: String,
    },
    /// Update bucket configuration
    Update {
        /// Name of the bucket
        name: String,

        /// Set public access (true = public, false = private/block all)
        #[arg(long)]
        public: Option<bool>,

        /// Enable or disable versioning
        #[arg(long)]
        versioning: Option<bool>,

        /// Set encryption mode (AES256 or aws:kms)
        #[arg(long)]
        encryption: Option<String>,

        /// Add tags (Key=Value)
        #[arg(long, value_parser = parse_key_val::<String, String>)]
        tags: Vec<(String, String)>,
    },
    /// Manage lifecycle rules
    Lifecycle {
        /// Name of the bucket
        name: String,

        /// Rule ID
        #[arg(long)]
        id: String,

        /// Prefix filter (default empty)
        #[arg(long, default_value = "")]
        prefix: String,

        /// Transitions JSON string (e.g. '[{"days": 30, "storage_class": "STANDARD_IA"}]')
        #[arg(long)]
        transitions: String,

        /// Expiration days
        #[arg(long)]
        expiration: Option<i32>,

        /// Enable rule (default true)
        #[arg(long, default_value = "true")]
        status: bool,
    },
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[derive(Subcommand)]
enum ObjectAction {
    /// List objects in a bucket
    List {
        /// Name of the bucket
        bucket: String,
    },
    /// Upload an object to a bucket
    Upload {
        /// Name of the bucket
        bucket: String,
        /// Path to the file to upload
        file: String,
        /// Optional key for the object (defaults to filename)
        #[arg(short, long)]
        key: Option<String>,
    },
    /// Delete an object from a bucket
    Delete {
        /// Name of the bucket
        bucket: String,
        /// Key of the object
        key: String,
    },
    /// Restore an archived object
    Restore {
        /// Name of the bucket
        bucket: String,
        /// Key of the object
        key: String,
    },
    /// Get object attributes
    Attributes {
        /// Name of the bucket
        bucket: String,
        /// Key of the object
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let client = client::create_client(cli.region, cli.profile).await;

    match cli.command {
        Commands::Bucket { action } => match action {
            BucketAction::List => {
                buckets::list_buckets(&client).await?;
            }
            BucketAction::Create { name, public, versioning, encryption, tags } => {
                // For create bucket, we might need a region if not globally provided, 
                // but for now we'll rely on the client's region or default.
                // However, create_bucket in buckets.rs expects a region string for constraint.
                // We'll fetch the region from the client config if possible, or default to us-east-1.
                let region = client.config().region().map(|r| r.as_ref()).unwrap_or("us-east-1");
                buckets::create_bucket(&client, &name, region, public, versioning, encryption, tags).await?;
            }
            BucketAction::Config { name } => {
                buckets::get_bucket_config(&client, &name).await?;
            }
            BucketAction::Update { name, public, versioning, encryption, tags } => {
                buckets::update_bucket(&client, &name, public, versioning, encryption, tags).await?;
            }
            BucketAction::Lifecycle { name, id, prefix, transitions, expiration, status } => {
                buckets::put_lifecycle_rule(&client, &name, &id, &prefix, &transitions, expiration, status).await?;
            }
        },
        Commands::Object { action } => match action {
            ObjectAction::List { bucket } => {
                objects::list_objects(&client, &bucket).await?;
            }
            ObjectAction::Upload { bucket, file, key } => {
                objects::upload_object(&client, &bucket, &file, key).await?;
            }
            ObjectAction::Delete { bucket, key } => {
                objects::delete_object(&client, &bucket, &key).await?;
            }
            ObjectAction::Restore { bucket, key } => {
                objects::restore_object(&client, &bucket, &key).await?;
            }
            ObjectAction::Attributes { bucket, key } => {
                objects::get_object_attributes(&client, &bucket, &key).await?;
            }
        },
    }

    Ok(())
}
