//! Reading the words in a picture. The engine itself is whatever the operating system provides
//! (see the `fin-ocr` crate); this is only the shape it has to fit, so the rest of the app can be
//! tested with a stand-in.

/// Reads the words in a picture. Implementations must work entirely on this computer.
pub trait Ocr: Send + Sync {
    /// Text in an image file (JPEG or PNG), or why it could not be read.
    fn read_image(&self, bytes: &[u8]) -> Result<String, String>;
}
