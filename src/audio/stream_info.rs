use gstreamer as gst;
use gstreamer_pbutils::{Discoverer, prelude::*};
use gtk::glib;

use crate::async_utils::spawn_tokio;

#[derive(Default, Debug)]
pub struct StreamInfo {
    pub codec: Option<String>,
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub bit_rate: Option<u32>,
    pub container_format: Option<String>,
    pub encoder: Option<String>,
}

pub fn discover_stream_info(uri: &str, callback: impl FnOnce(StreamInfo) + 'static) {
    let uri = uri.to_string();
    spawn_tokio(
        async move {
            tokio::task::spawn_blocking(move || -> Result<StreamInfo, glib::Error> {
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

                Ok(stream_info)
            })
            .await
            .unwrap_or(Ok(StreamInfo::default()))
        },
        move |result: Result<StreamInfo, glib::Error>| {
            let stream_info = result.unwrap_or_default();
            callback(stream_info);
        },
    );
}
