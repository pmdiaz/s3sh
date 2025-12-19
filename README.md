# S3sh - S3 Shell

A Rust command-line interface (CLI) application for managing Amazon S3 buckets and objects simply and elegantly.

## Features

- **Bucket Management**: List, create, and view configuration.
- **Object Management**: List, upload (with progress bar), delete, restore, and view attributes.
- **Lifecycle Management**: Manage lifecycle rules (transitions and expiration).
- **Friendly Interface**: Formatted output with colors and tables.
- **Flexible Configuration**: Support for AWS profiles and regions.

## Requirements

- Rust (cargo) installed.
- AWS credentials configured (via `aws configure` or environment variables).

## Installation and Build

To build the project:

```bash
cargo build --release
```

The resulting binary will be located at `target/release/s3sh`.

## Usage

The application can be run directly with `cargo run` or using the compiled binary.

### Global Options

- `-r, --region <REGION>`: Specify the AWS region (e.g., `us-east-1`).
- `-p, --profile <PROFILE>`: Specify the AWS profile to use.

### Bucket Commands

**List all buckets:**
```bash
cargo run -- bucket list
```

**Create a new bucket:**
```bash
cargo run -- bucket create <bucket-name>

# Create with initial configuration
cargo run -- bucket create <bucket-name> --public true --versioning true --tags Env=Dev
```

**View bucket configuration:**
```bash
cargo run -- bucket config <bucket-name>
```

**Update bucket configuration:**
```bash
# Make public (disable Block Public Access)
cargo run -- bucket update <bucket-name> --public true

# Enable versioning
cargo run -- bucket update <bucket-name> --versioning true

# Configure encryption (AES256 or aws:kms)
cargo run -- bucket update <bucket-name> --encryption AES256

# Add tags
cargo run -- bucket update <bucket-name> --tags Environment=Dev Project=S3sh
```

### Lifecycle Management

Manage lifecycle rules for a bucket.

```bash
# Add a lifecycle rule
s3sh bucket lifecycle <bucket-name> \
  --id <rule-id> \
  --transitions '[{"days": 30, "storage_class": "STANDARD_IA"}]' \
  --expiration 365
```

Arguments:
- `--id`: Unique identifier for the rule.
- `--transitions`: JSON array with transitions (e.g., `[{"days": 30, "storage_class": "STANDARD_IA"}]`).
- `--expiration`: (Optional) Days for object expiration.
- `--prefix`: (Optional) Prefix to filter affected objects.
- `--status`: (Optional) `true` to enable, `false` to disable (default: `true`).

### Object Commands

**List objects in a bucket:**
```bash
cargo run -- object list <bucket-name>
```

**Upload a file:**
```bash
cargo run -- object upload <bucket-name> <path-to-file>
# Optionally specify a different key (S3 name):
cargo run -- object upload <bucket-name> <path-to-file> --key <destination-name>
```

**View object attributes:**
```bash
cargo run -- object attributes <bucket-name> <object-key>
```

**Delete an object:**
```bash
cargo run -- object delete <bucket-name> <object-key>
```

**Restore an object (from Glacier):**
```bash
cargo run -- object restore <bucket-name> <object-key>
```

## Credential Configuration

The application uses the default AWS credential provider chain. It will look for credentials in this order:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, etc.).
2. Configuration files (`~/.aws/credentials`, `~/.aws/config`).


## Testing

The project includes unit and integration tests. To run them:

```bash
cargo test
```

Integration tests are located in the `tests/` directory and use mocks to simulate AWS S3 responses, so they do not require real credentials nor do they generate costs.
