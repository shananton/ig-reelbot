use async_process::Command;
use futures::StreamExt;
use std::io;
use std::process::Stdio;
use substring::Substring;
use teloxide::payloads::SendVideoSetters;
use teloxide::prelude::Requester;
use teloxide::types::InputFile;
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
    let bot = Bot::from_env();
    let mut listener = Polling::builder(bot.clone()).build();
    let mut stream = listener.as_stream();

    let mut last_id = 0;
    while let Some(result) = stream.next().await {
        let Ok(update) = result else { continue };
        if update.id <= last_id {
            continue;
        }
        last_id = update.id;

        let UpdateKind::Message(message) = update.kind else { continue };
        let MessageKind::Common(message_common) = message.kind else { continue };
        let MediaKind::Text(media_text) = message_common.media_kind else { continue };

        for entity in media_text.entities {
            if entity.kind != MessageEntityKind::Url {
                continue;
            }
            let url = media_text
                .text
                .substring(entity.offset, entity.offset + entity.length);
            let Ok(url) = Url::parse(url) else { continue };
            if url.domain() != Some(INSTAGRAM_DOMAIN) {
                continue;
            }
            let Some(mut iter) = url.path_segments() else { continue };
            if iter.next().unwrap() != REEL_PATH_SEGMENT {
                continue;
            }

            let bot = bot.clone();
            tokio::spawn(async move {
                if let Ok(video) = download_video(url.as_str()).await {
                    let _ = bot
                        .send_video(message.chat.id, InputFile::memory(video))
                        .reply_to_message_id(message.id)
                        .await;
                }
            });
        }
    }
}
