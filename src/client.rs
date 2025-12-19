use aws_sdk_s3::Client;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;

pub async fn create_client(region: Option<String>, profile: Option<String>) -> Client {
    let region_provider = RegionProviderChain::first_try(region.map(aws_sdk_s3::config::Region::new))
        .or_default_provider()
        .or_else(aws_sdk_s3::config::Region::new("us-east-1"));

    let mut config_loader = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider);

    if let Some(profile_name) = profile {
        config_loader = config_loader.profile_name(profile_name);
    }

    let config = config_loader.load().await;
    Client::new(&config)
}
