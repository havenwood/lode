use lode::{Resolver, RubyGemsClient};

#[test]
fn creates_resolver() {
    let client = RubyGemsClient::new("https://rubygems.org").unwrap();
    let _resolver = Resolver::new(client);
}
