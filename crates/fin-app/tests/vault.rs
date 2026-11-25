use fin_app::App;

const W2: &str = "Form W-2 Wage and Tax Statement 2025\nWages, tips, other compensation 85,000.00\nFederal income tax withheld 12,000.00\n";

#[test]
fn a_document_is_stored_recognised_and_read_back_byte_for_byte() {
    let a = App::in_memory().unwrap();
    let d = a.vault_add("My W-2.txt", W2.as_bytes()).unwrap();
    assert_eq!(d.kind, "W-2");
    assert_eq!(d.year, Some(2025));
    assert_eq!(d.size, W2.len() as i64);
    assert!(d
        .fields
        .iter()
        .any(|f| f.label == "Wages" && f.cents == 8_500_000));
    let (name, bytes) = a.vault_get(d.id).unwrap();
    assert_eq!(name, "My W-2.txt");
    assert_eq!(bytes, W2.as_bytes());
}

#[test]
fn binary_files_are_kept_without_text_and_listed_as_documents() {
    let a = App::in_memory().unwrap();
    let d = a
        .vault_add("scan.png", &[0x89, b'P', b'N', b'G', 0, 1, 2])
        .unwrap();
    assert_eq!(d.kind, "Document");
    assert!(d.fields.is_empty());
}

#[test]
fn search_matches_names_kinds_and_inner_text_and_delete_removes() {
    let a = App::in_memory().unwrap();
    let w2 = a.vault_add("w2.txt", W2.as_bytes()).unwrap();
    a.vault_add("note.txt", b"keep the roof receipts").unwrap();
    assert_eq!(a.vault_list("").unwrap().len(), 2);
    assert_eq!(a.vault_list("wage and tax").unwrap().len(), 1);
    assert_eq!(a.vault_list("W-2").unwrap()[0].id, w2.id);
    assert_eq!(a.vault_list("roof").unwrap().len(), 1);
    assert!(a.vault_list("nothing like this").unwrap().is_empty());
    a.vault_delete(w2.id).unwrap();
    assert_eq!(a.vault_list("").unwrap().len(), 1);
    assert!(a
        .vault_delete(w2.id)
        .unwrap_err()
        .contains("isn't in the vault"));
    assert!(a.vault_get(w2.id).is_err());
}

#[test]
fn bad_input_is_a_readable_error() {
    let a = App::in_memory().unwrap();
    assert!(a.vault_add(" ", b"x").unwrap_err().contains("name"));
    assert!(a.vault_add("a", b"").unwrap_err().contains("empty"));
    let big = vec![b'a'; fin_app::vault::MAX_BYTES + 1];
    assert!(a.vault_add("big", &big).unwrap_err().contains("25 MB"));
}

mod ocr {
    use super::W2;
    use fin_app::vault::Ocr;
    use fin_app::App;
    use lopdf::{dictionary, Document, Object, Stream};
    use std::sync::Arc;

    /// Pretends every picture is a scan of a W-2.
    struct FakeOcr;
    impl Ocr for FakeOcr {
        fn read_image(&self, bytes: &[u8]) -> Result<String, String> {
            if bytes.len() < 4 {
                return Err("too small".into());
            }
            Ok(W2.to_string())
        }
    }

    struct Broken;
    impl Ocr for Broken {
        fn read_image(&self, _: &[u8]) -> Result<String, String> {
            Err("engine failed".into())
        }
    }

    fn scanned_pdf(jpeg: Vec<u8>) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let img = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 2, "Height" => 2,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
            },
            jpeg,
        ));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => Object::Reference(img) } },
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let root = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", root);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn a_photo_is_read_when_an_engine_is_available() {
        let mut a = App::in_memory().unwrap();
        a.set_ocr(Some(Arc::new(FakeOcr)));
        let d = a
            .vault_add("w2.jpg", &[0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3])
            .unwrap();
        assert_eq!(d.kind, "W-2");
        assert!(d
            .fields
            .iter()
            .any(|f| f.label == "Wages" && f.cents == 8_500_000));
        // The words inside are searchable too.
        assert_eq!(a.vault_list("withheld").unwrap().len(), 1);
    }

    #[test]
    fn a_scanned_pdf_is_read_page_by_page() {
        let mut a = App::in_memory().unwrap();
        a.set_ocr(Some(Arc::new(FakeOcr)));
        let pdf = scanned_pdf(vec![0xFF, 0xD8, 0xFF, 0xE0, 9, 9, 9]);
        let d = a.vault_add("scan.pdf", &pdf).unwrap();
        assert_eq!(d.kind, "W-2");
        // The original file is stored untouched.
        assert_eq!(a.vault_get(d.id).unwrap().1, pdf);
    }

    #[test]
    fn without_an_engine_or_when_it_fails_the_file_is_still_kept() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
        let a = App::in_memory().unwrap();
        assert_eq!(a.vault_add("w2.jpg", &jpeg).unwrap().kind, "Document");
        assert_eq!(
            a.vault_add("scan.pdf", &scanned_pdf(jpeg.clone()))
                .unwrap()
                .kind,
            "Document"
        );
        let mut b = App::in_memory().unwrap();
        b.set_ocr(Some(Arc::new(Broken)));
        assert_eq!(b.vault_add("w2.jpg", &jpeg).unwrap().kind, "Document");
        b.set_ocr(None);
        assert_eq!(b.vault_add("again.jpg", &jpeg).unwrap().kind, "Document");
    }

    #[test]
    fn unreadable_scans_are_flagged() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3, 4];
        let mut a = App::in_memory().unwrap();
        assert!(!a.vault_add("scan.jpg", &jpeg).unwrap().has_text);
        a.set_ocr(Some(Arc::new(FakeOcr)));
        let row = a.vault_add("w2.jpg", &jpeg).unwrap();
        assert!(row.has_text);
        assert!(a
            .vault_list("")
            .unwrap()
            .iter()
            .any(|d| d.name == "w2.jpg" && d.has_text));
        assert!(a
            .vault_list("")
            .unwrap()
            .iter()
            .any(|d| d.name == "scan.jpg" && !d.has_text));
    }
}
