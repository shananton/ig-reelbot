use async_process::Command;
use futures::StreamExt;
use log::{error, info, trace, warn};
use std::io;
use std::process::Stdio;
use substring::Substring;
use teloxide::payloads::{SendMessageSetters, SendVideoSetters};
use teloxide::prelude::Requester;
use teloxide::types::{AllowedUpdate, InputFile};
use teloxide::{
    types::{MediaKind, MessageEntityKind, MessageKind, UpdateKind},
    update_listeners::{AsUpdateStream, Polling},
    Bot,
};
use url::Url;

const INSTAGRAM_DOMAIN: &str = "www.instagram.com";
const REEL_PATH_SEGMENT: &str = "reel";

const YTDLP_PATH: &str = "yt-dlp";

#[tokio::main]
async fn main() {
    async_main().await
}

async fn download_video(url: &str) -> io::Result<Vec<u8>> {
    let output = Command::new(YTDLP_PATH)
        .args(["-o", "-", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?
        .output()
        .await?;
    if !output.status.success() {
        return Err(io::Error::from(io::ErrorKind::Other));
    }
    Ok(output.stdout)
}

async fn async_main() {
    env_logger::init();

    info!("Starting reel bot...");

    let bot = Bot::from_env();
    let mut listener = Polling::builder(bot.clone())
        .allowed_updates(vec![AllowedUpdate::Message])
        .build();
    let mut stream = listener.as_stream();

    let mut last_id = 0;
    trace!("Starting event loop...");
    while let Some(result) = stream.next().await {
        trace!("Update or error received: {result:?}");
        let update = match result {
            Ok(update) => update,
            Err(err) => {
                error!("Request error: {err}");
                continue;
            }
        };
        if update.id <= last_id {
            info!("Duplicate update received with update.id={}", update.id);
            continue;
        }
        last_id = update.id;

        let UpdateKind::Message(message) = update.kind else {
            warn!("Unexpected update kind received: {:?}", update.kind);
            continue
        };
        let MessageKind::Common(message_common) = message.kind else { continue };
        let MediaKind::Text(media_text) = message_common.media_kind else { continue };

        for entity in media_text.entities {
            if entity.kind != MessageEntityKind::Url {
                continue;
            }
            let url = media_text
                .text
                .substring(entity.offset, entity.offset + entity.length);
            let Ok(url) = Url::parse(url) else {
                warn!("Failed to parse a URL in a message: {url}");
                continue;
            };
            if url.domain() != Some(INSTAGRAM_DOMAIN) {
                continue;
            }
            let Some(mut iter) = url.path_segments() else { continue };
            if iter.next().unwrap() != REEL_PATH_SEGMENT {
                continue;
            }

            let bot = bot.clone();
            tokio::spawn(async move {
                match download_video(url.as_str()).await {
                    Ok(video) => {
                        if let Err(err) = bot
                            .send_video(message.chat.id, InputFile::memory(video))
                            .reply_to_message_id(message.id)
                            .await
                        {
                            error!("Error sending video: {err}");
                        }
                    }
                    Err(err) => {
                        warn!("Failed to download video with yt-dlp: {err}");
                        const DOWNLOAD_ERROR_TEXT: &str =
                            "Sorry, I wasn't able to download that 🥲";
                        if let Err(err) = bot
                            .send_message(message.chat.id, DOWNLOAD_ERROR_TEXT)
                            .reply_to_message_id(message.id)
                            .await
                        {
                            error!("Error sending response message: {err}");
                        }
                    }
                }
            });
        }
    }

    error!("Update stream ended unexpectedly, shutting down...")
}
