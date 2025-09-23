use gtk::{
    gdk::Texture,
    gdk_pixbuf::{PixbufLoader, prelude::PixbufLoaderExt},
    glib,
};

pub fn bytes_to_texture(image_data: &[u8]) -> Result<Texture, glib::Error> {
    let loader = PixbufLoader::new();
    loader.write(image_data)?;
    loader.close()?;
    match loader.pixbuf() {
        Some(pixbuf) => Ok(Texture::for_pixbuf(&pixbuf)),
        None => Err(glib::Error::new(
            glib::FileError::Failed,
            "Failed to create pixbuf from image data",
        )),
    }
}
