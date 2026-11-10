use crate::{config, i18n::tr};
use gtk::glib;

#[derive(Debug, Clone, thiserror::Error, glib::Boxed)]
#[boxed_type(name = "GellyAudioError")]
pub enum AudioError {
    #[error("{0}")]
    Stream(#[source] glib::Error),
    #[error("{0}")]
    Resource(#[source] glib::Error),
    #[error("{0}")]
    Playback(#[source] glib::Error),
    #[error("Failed to seek")]
    SeekFailed,
}

impl AudioError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Stream(_) => tr("Unable to load the stream. Is the network available?"),
            Self::Resource(cause) | Self::Playback(cause) => {
                tr("Playback error: {}").replace("{}", &cause.to_string())
            }
            Self::SeekFailed => {
                if config::get_backend_type() == config::BackendType::Subsonic {
                    tr("Subsonic does not support seeking transcoded streams.")
                } else {
                    tr("Failed to seek")
                }
            }
        }
    }
}

impl From<glib::Error> for AudioError {
    fn from(cause: glib::Error) -> Self {
        if cause.kind::<gstreamer::StreamError>().is_some() {
            Self::Stream(cause)
        } else if cause.kind::<gstreamer::ResourceError>().is_some() {
            Self::Resource(cause)
        } else {
            Self::Playback(cause)
        }
    }
}
