mod common;
use common::{TestCase, MAGNET_LINK};

#[tokio::test]
async fn test_download() {
    let test_case = TestCase::new().await;
    test_case.send("/download").await;
    let wants = r#"Send torrent link or attach torrent file

Buttons:

/back"#;
    test_case.check(wants);
    test_case.send(MAGNET_LINK).await;
    test_case.check("OK");
    test_case.send(MAGNET_LINK).await;
    // item is already added
    test_case.check("FAIL");
}
