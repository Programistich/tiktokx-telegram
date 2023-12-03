use std::hash::Hash;
use std::process::Command;
use frankenstein::{AsyncTelegramApi, ChatAction, FileUpload, InputFile, Message, SendChatActionParams, SendVideoParams};
use frankenstein::GetUpdatesParams;
use frankenstein::{AsyncApi, UpdateContent};

// https://vm.tiktok.com/ZM6e3Yxy6
static TIKTOK_REGEX: &str = r"https://vm\.tiktok\.com/[A-Za-z0-9]+";

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
    let text = message.text.unwrap();

    let tiktok_regex = regex::Regex::new(TIKTOK_REGEX).unwrap();
    if !tiktok_regex.is_match(&*text) {
        log::info!("Not a tiktok url {}", text);
        return;
    }

    log::info!("Downloading tiktok {}", text);

    let send_typing_params = SendChatActionParams::builder()
        .chat_id(message.chat.id)
        .action(ChatAction::UploadVideo)
        .build();

    if let Err(err) = api.send_chat_action(&send_typing_params).await {
        println!("Failed to send message: {err:?}");
    }

    let uuid = uuid::Uuid::new_v4().to_string();
    let name_file = "./video/".to_owned() + &*uuid + ".mp4";
    let file = std::path::Path::new(&*name_file);

    let yt_dlp_path = std::env::var("YT_DLP")
        .expect("YT_DLP not set in env");

    let output = Command::new(yt_dlp_path)
        .args(["-v", &text])
        .args(["-o", &name_file])
        .output()
        .expect("failed to execute process");
    println!("output: {}", String::from_utf8_lossy(&output.stdout));

    let send_video_params = SendVideoParams::builder()
        .chat_id(message.chat.id)
        .video(FileUpload::InputFile(
            InputFile {
                path: file.to_path_buf(),
            }
        ))
        .reply_to_message_id(message.message_id)
        .build();

    if let Err(err) = api.send_video(&send_video_params).await {
        println!("Failed to send message: {err:?}");
    }

    std::fs::remove_file(file).expect("Failed to remove video file");
}