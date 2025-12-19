use s3sh::buckets::{create_bucket, validate_bucket_name};
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Region, BehaviorVersion};
use aws_smithy_runtime::client::http::test_util::StaticReplayClient;
use aws_smithy_types::body::SdkBody;

#[test]
fn test_validate_bucket_name() {
    assert!(validate_bucket_name("valid-bucket-name").is_ok());
    assert!(validate_bucket_name("123.456").is_ok());
    
    assert!(validate_bucket_name("UPPERCASE").is_err());
    assert!(validate_bucket_name("sh").is_err()); // Too short
    assert!(validate_bucket_name("-starts-with-hyphen").is_err());
    assert!(validate_bucket_name("ends-with-hyphen-").is_err());
    assert!(validate_bucket_name("contains space").is_err());
}

#[tokio::test]
async fn test_create_bucket_simple() {
    let http_client = StaticReplayClient::new(vec![
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/my-test-bucket")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .unwrap(),
        )
    ]);

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .build();
    
    let client = Client::from_conf(config);

    let result = create_bucket(&client, "my-test-bucket", "us-east-1", None, None, None, vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_bucket_with_config() {
    let http_client = StaticReplayClient::new(vec![
        // Create bucket request
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/config-bucket")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .unwrap(),
        ),
        // Put public access block
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/config-bucket?publicAccessBlock")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .unwrap(),
        ),
        // Put bucket versioning
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/config-bucket?versioning")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .unwrap(),
        ),
    ]);

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .build();
    
    let client = Client::from_conf(config);

    // Test with public=true and versioning=true
    let result = create_bucket(
        &client, 
        "config-bucket", 
        "us-east-1", 
        Some(true), 
        Some(true), 
        None, 
        vec![]
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_put_lifecycle_rule() {
    let http_client = StaticReplayClient::new(vec![
        // Get bucket lifecycle config (returns NoSuchLifecycleConfiguration initially)
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://s3.us-east-1.amazonaws.com/lifecycle-bucket?lifecycle")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(404)
                .body(SdkBody::from(r#"<?xml version="1.0" encoding="UTF-8"?>
                    <Error>
                        <Code>NoSuchLifecycleConfiguration</Code>
                        <Message>The lifecycle configuration does not exist</Message>
                        <RequestId>REQ123</RequestId>
                        <HostId>HOST123</HostId>
                    </Error>"#))
                .unwrap(),
        ),
        // Put bucket lifecycle config
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/lifecycle-bucket?lifecycle")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::empty())
                .unwrap(),
        ),
    ]);

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .build();
    
    let client = Client::from_conf(config);

    let transitions_json = r#"[{"days": 30, "storage_class": "STANDARD_IA"}]"#;

    let result = s3sh::buckets::put_lifecycle_rule(
        &client, 
        "lifecycle-bucket", 
        "rule-1", 
        "logs/", 
        transitions_json, 
        Some(365), 
        true
    ).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_put_lifecycle_rule_invalid_json() {
    let http_client = StaticReplayClient::new(vec![]); // No requests expected

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .build();
    
    let client = Client::from_conf(config);

    let transitions_json = r#"invalid-json"#;

    let result = s3sh::buckets::put_lifecycle_rule(
        &client, 
        "lifecycle-bucket", 
        "rule-1", 
        "logs/", 
        transitions_json, 
        Some(365), 
        true
    ).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid transitions JSON"));
}
