use glycin::{ErrorCtx, Loader};
use gtk::gdk::Texture;

pub async fn bytes_to_texture(image_data: &[u8]) -> Result<Texture, ErrorCtx> {
    let image = Loader::new_vec(image_data.to_vec()).load().await?;
    let texture = image.next_frame().await?.texture();
    Ok(texture)
}
