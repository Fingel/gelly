use glycin::Loader;
use gtk::{gdk::Texture, glib};

pub async fn bytes_to_texture(image_data: &[u8]) -> Result<Texture, glib::Error> {
    let image = Loader::new_vec(image_data.to_vec())
        .load()
        .await
        .expect("Failed to parse image data.");
    let texture = image.next_frame().await.unwrap().texture();
    Ok(texture)
}
