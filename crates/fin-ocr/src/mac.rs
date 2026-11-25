//! Apple's Vision framework.
use fin_ingest::ocr::Ocr;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSData, NSDictionary};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

pub struct Vision;

impl Ocr for Vision {
    fn read_image(&self, bytes: &[u8]) -> Result<String, String> {
        let data = NSData::with_bytes(bytes);
        let options = NSDictionary::new();
        let handler = VNImageRequestHandler::initWithData_options(
            VNImageRequestHandler::alloc(),
            &data,
            &options,
        );
        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);
        let as_request: Retained<VNRequest> =
            Retained::into_super(Retained::into_super(request.clone()));
        let requests = NSArray::from_retained_slice(&[as_request]);
        handler
            .performRequests_error(&requests)
            .map_err(|e| e.localizedDescription().to_string())?;
        let Some(results) = request.results() else {
            return Ok(String::new());
        };
        let mut lines = Vec::new();
        for obs in results.iter() {
            if let Some(best) = obs.topCandidates(1).iter().next() {
                lines.push(best.string().to_string());
            }
        }
        Ok(lines.join("\n"))
    }
}
