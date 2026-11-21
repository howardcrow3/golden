use fin_ai::manager::{installed_path, remove, status, DownloadState};
use fin_ai::{Catalog, ModelEntry, Role};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn serve(body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/m.gguf", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = sock.read(&mut buf).await.unwrap();
            let mut r = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            r.extend_from_slice(&body);
            let _ = sock.write_all(&r).await;
            let _ = sock.shutdown().await;
        }
    });
    url
}

fn entry(url: &str, body: &[u8]) -> ModelEntry {
    ModelEntry {
        id: "t".into(),
        role: Role::General,
        name: "Test".into(),
        file: "m.gguf".into(),
        size_bytes: body.len() as u64,
        license: "MIT".into(),
        min_ram_gb: 1,
        default: true,
        url: Some(url.into()),
        sha256: Some(
            Sha256::digest(body)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        ),
    }
}

#[test]
fn the_recommended_model_follows_memory_and_reports_whether_it_is_installed() {
    let dir = tempfile::tempdir().unwrap();
    let c = Catalog::builtin();
    let s = status(&c, 8, dir.path()).unwrap();
    assert_eq!(
        (s.id.as_str(), s.installed),
        ("qwen3-4b-instruct-q4km", false)
    );
    assert_eq!(s.size_mb, 2497);
    assert_eq!(
        status(&c, 16, dir.path()).unwrap().id,
        "qwen3-8b-instruct-q4km"
    );
    assert!(status(&c, 4, dir.path()).unwrap_err().contains("memory"));
    std::fs::write(dir.path().join("qwen3-4b-instruct-q4_k_m.gguf"), b"x").unwrap();
    assert!(status(&c, 8, dir.path()).unwrap().installed);
    assert!(installed_path(&c, "qwen3-4b-instruct-q4km", dir.path()).is_some());
    assert!(installed_path(&c, "nope", dir.path()).is_none());
}

#[tokio::test]
async fn a_download_reports_progress_installs_and_frees_the_slot() {
    let body: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    let url = serve(body.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let e = entry(&url, &body);
    let st = Arc::new(DownloadState::default());
    st.begin(&e).unwrap();
    assert!(st.progress().running);
    assert!(st.begin(&e).unwrap_err().contains("already running"));
    st.run(&e, dir.path()).await;
    let p = st.progress();
    assert_eq!(
        (p.running, p.downloaded_mb, p.total_mb, p.error.as_str()),
        (false, 3, 3, "")
    );
    assert_eq!(std::fs::read(dir.path().join("m.gguf")).unwrap(), body);
    // The slot is free again.
    st.begin(&e).unwrap();
}

#[tokio::test]
async fn a_wrong_checksum_is_reported_not_installed() {
    let body = vec![7u8; 10_000];
    let url = serve(body.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let mut e = entry(&url, &body);
    e.sha256 = Some("0".repeat(64));
    let st = DownloadState::default();
    st.begin(&e).unwrap();
    st.run(&e, dir.path()).await;
    assert!(
        st.progress().error.contains("checksum"),
        "{}",
        st.progress().error
    );
    assert!(!dir.path().join("m.gguf").exists());
    assert!(!st.progress().running);
}

#[tokio::test]
async fn cancelling_is_not_an_error_and_keeps_the_partial_for_later() {
    let body = vec![1u8; 5_000_000];
    let url = serve(body.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let e = entry(&url, &body);
    let st = DownloadState::default();
    st.begin(&e).unwrap();
    st.cancel(); // before the first chunk is read
    st.run(&e, dir.path()).await;
    assert_eq!(st.progress().error, "");
    assert!(!dir.path().join("m.gguf").exists());
    // A fresh start finishes the job.
    st.begin(&e).unwrap();
    st.run(&e, dir.path()).await;
    assert!(dir.path().join("m.gguf").exists());
}

#[test]
fn removing_deletes_the_model_and_its_partial_and_tolerates_absence() {
    let dir = tempfile::tempdir().unwrap();
    let c = Catalog::builtin();
    let f = dir.path().join("qwen3-4b-instruct-q4_k_m.gguf");
    std::fs::write(&f, b"x").unwrap();
    std::fs::write(
        dir.path().join("qwen3-4b-instruct-q4_k_m.gguf.partial"),
        b"x",
    )
    .unwrap();
    remove(&c, "qwen3-4b-instruct-q4km", dir.path()).unwrap();
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    remove(&c, "qwen3-4b-instruct-q4km", dir.path()).unwrap();
    assert!(remove(&c, "nope", dir.path()).is_err());
}
