//! Windows' built-in OCR (Windows.Media.Ocr), using the languages in the person's profile.
use fin_ingest::ocr::Ocr;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

pub struct WindowsOcr;

fn read(bytes: &[u8]) -> windows::core::Result<String> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(bytes)?;
    writer.StoreAsync()?.get()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
    let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;
    Ok(result.Text()?.to_string())
}

impl Ocr for WindowsOcr {
    fn read_image(&self, bytes: &[u8]) -> Result<String, String> {
        // The Windows runtime wants its own thread rather than one that may be a UI thread.
        let owned = bytes.to_vec();
        std::thread::spawn(move || read(&owned).map_err(|e| e.message()))
            .join()
            .map_err(|_| "Text recognition stopped unexpectedly.".to_string())?
    }
}
