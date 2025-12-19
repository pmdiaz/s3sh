use s3sh::objects::{list_objects, upload_object, delete_object};
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Region, BehaviorVersion};
use aws_smithy_runtime::client::http::test_util::StaticReplayClient;
use aws_smithy_types::body::SdkBody;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_list_objects() {
    let http_client = StaticReplayClient::new(vec![
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("GET")
                .uri("https://s3.us-east-1.amazonaws.com/test-bucket?list-type=2")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(200)
                .body(SdkBody::from(r#"<?xml version="1.0" encoding="UTF-8"?>
                    <ListBucketResult>
                        <Name>test-bucket</Name>
                        <Contents>
                            <Key>file1.txt</Key>
                            <Size>1024</Size>
                            <LastModified>2023-01-01T00:00:00.000Z</LastModified>
                        </Contents>
                    </ListBucketResult>"#))
                .unwrap(),
        )
    ]);

    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .build();
    
    let client = Client::from_conf(config);

    let result = list_objects(&client, "test-bucket").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_upload_object() {
    let http_client = StaticReplayClient::new(vec![
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("PUT")
                .uri("https://s3.us-east-1.amazonaws.com/test-bucket/test-file.txt")
                .body(SdkBody::from("hello world"))
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

    // Create a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "hello world").unwrap();
    let path = temp_file.path().to_str().unwrap();

    let result = upload_object(&client, "test-bucket", path, Some("test-file.txt".to_string())).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_object() {
    let http_client = StaticReplayClient::new(vec![
        aws_smithy_runtime::client::http::test_util::ReplayEvent::new(
            http::Request::builder()
                .method("DELETE")
                .uri("https://s3.us-east-1.amazonaws.com/test-bucket/file-to-delete.txt")
                .body(SdkBody::empty())
                .unwrap(),
            http::Response::builder()
                .status(204)
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

    let result = delete_object(&client, "test-bucket", "file-to-delete.txt").await;
    assert!(result.is_ok());
}
