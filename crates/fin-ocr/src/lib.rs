//! Text recognition (OCR) with whatever the operating system already provides, so scans and
//! photos are read on this computer and nothing is sent anywhere: Apple's Vision framework on a
//! Mac, Windows' built-in OCR on Windows. Other systems get no engine, and the vault simply keeps
//! such files without reading them.
use fin_ingest::ocr::Ocr;
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod win;

/// The engine for this operating system, if it has one.
pub fn system_engine() -> Option<Arc<dyn Ocr>> {
    #[cfg(target_os = "macos")]
    {
        Some(Arc::new(mac::Vision))
    }
    #[cfg(target_os = "windows")]
    {
        Some(Arc::new(win::WindowsOcr))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_engine_exists_on_the_platforms_that_have_one() {
        let e = super::system_engine();
        assert_eq!(
            e.is_some(),
            cfg!(any(target_os = "macos", target_os = "windows"))
        );
    }
}
