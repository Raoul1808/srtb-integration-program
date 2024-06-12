use web_sys::{
    js_sys::{Array, Promise, Uint8Array},
    wasm_bindgen::{closure::Closure, JsCast, JsValue},
    Blob, BlobPropertyBag, FileReader, Url,
};

use super::ReadFile;

pub fn alert(msg: &str) {
    let window = web_sys::window().unwrap();
    window.alert_with_message(msg).expect("alert() failed");
}

pub async fn open_file(ext: &str) -> Option<ReadFile> {
    let ext = if !ext.starts_with(".") {
        format!(".{}", ext)
    } else {
        ext.into()
    };
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let input = document
        .create_element("input")
        .expect("document.create_element() failed")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("cast to HtmlElement failed");
    input.set_hidden(true);
    input.set_accept(&ext);
    input.set_type("file");
    let input_clone = input.clone();
    let promise = Promise::new(&mut move |res, rej| {
        let listener = Closure::once_into_js(Box::new(move || {
            res.call0(&JsValue::undefined()).unwrap();
        }) as Box<dyn FnMut()>);
        input_clone
            .add_event_listener_with_callback("change", listener.as_ref().unchecked_ref())
            .expect("element.addEventListener() failed");
        let listener = Closure::once_into_js(Box::new(move || {
            rej.call0(&JsValue::undefined()).unwrap();
        }) as Box<dyn FnMut()>);
        input_clone
            .add_event_listener_with_callback("cancel", listener.as_ref().unchecked_ref())
            .expect("element.addEventListener() failed");
    });
    input.click();
    let future = wasm_bindgen_futures::JsFuture::from(promise);
    future.await.ok()?;
    let file = input.files().expect("input.files failed");
    let file = file.item(0)?;
    let file_reader = FileReader::new().expect("new FileReader() failed");
    let file_reader_clone = file_reader.clone();
    let promise = Promise::new(&mut move |res, _rej| {
        let file_reader_clone_clone = file_reader_clone.clone();
        let listener = Closure::once_into_js(Box::new(move || {
            res.call1(
                &JsValue::undefined(),
                &file_reader_clone_clone
                    .result()
                    .expect("FileReader.result failed"),
            )
            .unwrap();
        }) as Box<dyn FnMut()>);
        file_reader_clone.set_onload(Some(listener.as_ref().unchecked_ref()));
    });
    file_reader
        .read_as_text(&file)
        .expect("FileReader.readAsText() failed");
    let future = wasm_bindgen_futures::JsFuture::from(promise);
    let result = future.await.unwrap();
    Some(ReadFile {
        name: file.name(),
        content: result.as_string().unwrap(),
    })
}

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
