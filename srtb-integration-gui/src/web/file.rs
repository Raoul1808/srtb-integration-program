use web_sys::{
    js_sys::{Array, Uint8Array},
    wasm_bindgen::JsCast,
    Blob, BlobPropertyBag, Url,
};

pub fn save_file(filename: &str, data: &[u8]) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let array = Array::new();
    let bytes = Uint8Array::new(&unsafe { Uint8Array::view(data) }.into());
    array.push(&bytes.buffer());
    let blob = Blob::new_with_u8_array_sequence_and_options(
        &array,
        BlobPropertyBag::new().type_("application/octet-stream"),
    )
    .expect("new Blob() failed");
    let url = Url::create_object_url_with_blob(&blob).expect("URL.createObjectUrl() failed");
    let download_link = document
        .create_element("a")
        .expect("document.create_element() failed")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("cast to HtmlElement failed");
    download_link.set_hidden(true);
    download_link.set_href(&url);
    download_link.set_download(filename);
    download_link.click();
    download_link.remove();
}
