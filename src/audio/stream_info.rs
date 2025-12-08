use gstreamer as gst;
use gstreamer_pbutils::{Discoverer, prelude::*};
use gtk::glib;
use thiserror::Error;

use crate::{
    async_utils::spawn_tokio,
    jellyfin::{Jellyfin, JellyfinError},
};

#[derive(Error, Debug)]
pub enum StreamInfoError {
    #[error("glib error: {0}")]
    Glib(#[from] glib::Error),

    #[error("Jellyfin error: {0}")]
    Jellyfin(#[from] JellyfinError),
}

#[derive(Default, Debug)]
pub struct StreamInfo {
    // Gstreamer properties
    pub codec: Option<String>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub bit_rate: Option<u32>,
    pub container_format: Option<String>,
    pub encoder: Option<String>,

    // Jellyfin Properties
    pub original_codec: Option<String>,
    pub original_bit_rate: Option<u64>,
    pub original_sample_rate: Option<u64>,
    pub original_channels: Option<u32>,
    pub original_container_format: Option<String>,
    pub file_size: Option<u64>,
    pub supports_direct_stream: Option<bool>,
    pub supports_direct_play: Option<bool>,
    pub supports_transcoding: Option<bool>,
}

pub fn discover_stream_info(
    uri: &str,
    song_id: &str,
    jellyfin: &Jellyfin,
    callback: impl FnOnce(StreamInfo) + 'static,
) {
    let uri = uri.to_string();
    let item_id = song_id.to_string();
    let jellyfin = jellyfin.clone();

    spawn_tokio(
        async move {
            let jellyfin_info = jellyfin.get_playback_info(&item_id).await?;
            tokio::task::spawn_blocking(move || -> Result<StreamInfo, StreamInfoError> {
                let discoverer = Discoverer::new(gst::ClockTime::from_seconds(10))?;
                let info = discoverer.discover_uri(&uri)?;
                let mut stream_info = StreamInfo::default();

                if let Some(container_stream) = info.container_streams().first()
                    && let Some(caps) = container_stream.caps()
                    && let Some(structure) = caps.structure(0)
                {
                    stream_info.container_format = Some(structure.name().to_string());
                }

                if let Some(audio_stream) = info.audio_streams().first()
                    && let Some(caps) = audio_stream.caps()
                {
                    if let Some(structure) = caps.structure(0) {
                        // Codec
                        stream_info.codec = Some(structure.name().to_string());
                        // Sample Rate
                        if let Ok(rate) = structure.get::<i32>("rate") {
                            stream_info.sample_rate = Some(rate);
                        }
                        // Channels
                        if let Ok(channels) = structure.get::<i32>("channels") {
                            stream_info.channels = Some(channels);
                        }
                    }

                    if let Some(tags) = audio_stream.tags() {
                        // Bit Rate
                        if let Some(bitrate_tag) = tags.get::<gst::tags::Bitrate>() {
                            stream_info.bit_rate = Some(bitrate_tag.get());
                        }
                        // Encoder information
                        if let Some(encoder_tag) = tags.get::<gst::tags::Encoder>() {
                            stream_info.encoder = Some(encoder_tag.get().to_string());
                        }
                    }
                }

                // Jellyfin stuff
                if let Some(media_source) = jellyfin_info.media_sources.first() {
                    stream_info.original_container_format = media_source.container.clone();
                    stream_info.file_size = media_source.size;
                    stream_info.supports_direct_play = media_source.supports_direct_play;
                    stream_info.supports_direct_stream = media_source.supports_direct_stream;
                    stream_info.supports_transcoding = media_source.supports_transcoding;

                    for media_stream in &media_source.media_streams {
                        if media_stream.type_ == Some("Audio".to_string()) {
                            stream_info.original_codec = media_stream.codec.clone();
                            stream_info.original_bit_rate = media_stream.bit_rate;
                            stream_info.original_sample_rate = media_stream.sample_rate;
                            stream_info.original_channels = media_stream.channels;
                            break;
                        }
                    }
                }

                Ok(stream_info)
            })
            .await
            .unwrap_or(Ok(StreamInfo::default()))
        },
        move |result: Result<StreamInfo, StreamInfoError>| {
            let stream_info = result.unwrap_or_default();
            callback(stream_info);
        },
    );
}
