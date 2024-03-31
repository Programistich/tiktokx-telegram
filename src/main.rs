use std::fs::File;
use std::io;
use std::process::Command;
use frankenstein::{AsyncTelegramApi, ChatAction, FileUpload, InputFile, Message, SendChatActionParams, SendVideoParams, SendMessageParams, ReplyParameters, ReactionTypeEmoji, ReactionType, SetMessageReactionParams, ChatId, SendPhotoParams};
use frankenstein::GetUpdatesParams;
use frankenstein::{AsyncApi, UpdateContent};
use frankenstein::MessageEntityType::Url;

// https://vm.tiktok.com/ZM6e3Yxy6 https://www.instagram.com/reel/C0ZVcxvsuWI/
static REGEX: &str = r"https://vm\.tiktok\.com/[A-Za-z0-9]+|https://www.instagram.com/reel/[A-Za-z0-9]+";
static AUTHOR_ID: u64 = 241629528;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Starting telegram bot");

    // For download videos
    create_output_dir().await;

    let telegram_token = std::env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN not set in env");
    let api = AsyncApi::new(&*telegram_token);

    let update_params_builder = GetUpdatesParams::builder();
    let mut update_params = update_params_builder.clone().build();

    loop {
        let result = api.get_updates(&update_params).await;
        match result {
            Ok(response) => {
                for update in response.result {
                    let content = update.content;
                    let api_clone = api.clone();

                    match content {
                        UpdateContent::Message(message) => {
                            tokio::spawn(async move {
                                process_message(message, api_clone).await;
                            });
                        }
                        _ => {}
                    }

                    update_params = update_params_builder
                        .clone()
                        .offset(update.update_id + 1)
                        .build();
                }
            }
            Err(error) => {
                println!("Failed to get updates: {error:?}");
            }
        }
    }
}

async fn create_output_dir() {
    let output_dir = std::path::Path::new("./video");

    // Delete old videos
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir).expect("Failed to remove video dir");
    }

    if !output_dir.exists() {
        std::fs::create_dir(output_dir).expect("Failed to create video dir");
    }
}

async fn process_message(message: Message, api: AsyncApi) {
    // check exist file in message
    if
        message.document.is_some() &&
        message.from.as_ref().unwrap().id == AUTHOR_ID &&
        message.document.as_ref().unwrap().file_name.clone().unwrap() == "cookies.txt" &&
        message.chat.id == message.from.as_ref().unwrap().id as i64
    {
        process_update_cookies(message, &api).await;
        return;
    }

    let text  = message.text.clone();

    if message.text.is_none() {
        return;
    }

    if message.text.as_ref().unwrap().contains("/air") {
        process_air_alert(message, &api).await;
        return;
    }

    let urls = message.entities.as_ref().map(|entities| {
        entities
            .iter()
            .filter(|entity| Url == entity.type_field)
            .map(move |entity| {
                let start = entity.offset as usize;
                let end = start + entity.length as usize;
                text.as_ref().unwrap()[start..end].to_string()
            })
    });

    match urls {
        Some(urls) => {
            for url in urls {
                if regex::Regex::new(REGEX).unwrap().is_match(&url) {
                    process_video(message, &api, &url).await;
                    return;
                }
            }
        },
        _ => {
            println!("No urls found");
        }
    }
}

async fn process_air_alert(message: Message, api: &AsyncApi) {
    let chat_id = ChatId::Integer(message.chat.id);
    let send_photo_params = SendPhotoParams::builder()
        .chat_id(chat_id)
        .photo(FileUpload::String("https://alerts.com.ua/map.png".to_string()))
        .build();
    let _ = api.send_photo(&send_photo_params).await;
}

async fn process_update_cookies(message: Message, api: &AsyncApi) {
    let file_id = message.document.as_ref().unwrap().file_id.clone();
    let get_file_params = frankenstein::GetFileParams::builder()
        .file_id(file_id.clone())
        .build();

    let file = api.get_file(&get_file_params).await;
    if file.is_err() {
        println!("Failed to get file: {file:?}");
        return;
    }
    let file_path = file.unwrap().result.file_path.unwrap();

    let telegram_token = std::env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN not set in env");
    let download_url = format!("https://api.telegram.org/file/bot{}/{}", telegram_token, file_path);

    let resp = reqwest::get(download_url).await.expect("request failed");
    let body = resp.text().await.expect("body invalid");

    let mut out = File::create("cookies.txt").expect("failed to create file");
    io::copy(&mut body.as_bytes(), &mut out).expect("failed to copy content");

    let chat_id = ChatId::Integer(message.chat.id);
    let send_message_params = SendMessageParams::builder()
        .chat_id(chat_id)
        .text("Cookies updated")
        .build();
    let _ = api.send_message(&send_message_params).await;
}

async fn process_video(message: Message, api: &AsyncApi, url: &String) {
    println!("Downloading Video {}", url);

    let send_typing_params = SendChatActionParams::builder()
        .chat_id(message.chat.id)
        .action(ChatAction::UploadVideo)
        .build();

    if let Err(err) = api.send_chat_action(&send_typing_params).await {
        send_react(&api, &message, "👎").await;
        println!("Failed to send message: {err:?}");
    }

    let uuid = uuid::Uuid::new_v4().to_string();
    let name_file = "./video/".to_owned() + &*uuid + ".mp4";
    let file = std::path::Path::new(&*name_file);

    let yt_dlp_path = std::env::var("YT_DLP")
        .expect("YT_DLP not set in env");

    let output = Command::new(yt_dlp_path)
        .args(["-v", &url])
        .args(["-o", &name_file])
        .args(["--cookies", "cookies.txt"])
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        send_react(&api, &message, "👎").await;
        if error.contains("login required") {
            send_author_info(&api, message.chat.id).await;
        } else {
            println!("Error: {}", error);
        }
    }

    let reply_params = ReplyParameters::builder()
        .message_id(message.message_id)
        .build();

    let send_video_params = SendVideoParams::builder()
        .chat_id(message.chat.id)
        .video(FileUpload::InputFile(
            InputFile {
                path: file.to_path_buf(),
            }
        ))
        .reply_parameters(reply_params)
        .build();

    if let Err(err) = api.send_video(&send_video_params).await {
        send_react(&api, &message, "👎").await;
        println!("Error sending video: {err:?}");
    }
    else {
        send_react(&api, &message, "👍").await;
        println!("Video sent");

    }
    match std::fs::remove_file(file) {
        Ok(_) => {
            println!("Video deleted");
        }
        Err(err) => {
            println!("Video not deleted with error: {err:?}");
        }
    }
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
}

async fn send_author_info(api: &AsyncApi, chat_id: i64)  {
    let chat_id = ChatId::Integer(chat_id);

    let send_message_params = SendMessageParams::builder()
        .chat_id(chat_id)
        .text("Update cookies")
        .build();
    let _ = api.send_message(&send_message_params).await;
}

async fn send_react(api: &AsyncApi, message: &Message, reaction: &str)  {
    let reaction = ReactionTypeEmoji { emoji: String::from(reaction) };
    let reaction_type = ReactionType::Emoji(reaction);

    let send_react_params = SetMessageReactionParams::builder()
        .chat_id(message.chat.id)
        .message_id(message.message_id)
        .reaction(vec![reaction_type])
        .is_big(true)
        .build();

    let _ = api.set_message_reaction(&send_react_params).await;
}