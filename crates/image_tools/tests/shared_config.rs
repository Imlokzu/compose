#[test]
fn public_endpoint_never_contains_a_provider_credential() {
    for url in [
        "https://service.example/v1/background/remove",
        "http://127.0.0.1:8787/v1/background/remove",
    ] {
        assert!(image_tools::validate_service_url(url).is_ok());
    }
    for url in [
        "http://service.example/remove",
        "https://key:secret@service.example/remove",
        "https://service.example/remove?key=secret",
        "file:///tmp/file",
    ] {
        assert!(image_tools::validate_service_url(url).is_err());
    }
}
