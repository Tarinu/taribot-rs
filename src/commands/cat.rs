use crate::CatConfig;
use log::debug;
use rand::{seq::IteratorRandom, thread_rng};
use serenity::{
    framework::standard::{
        macros::{check, command},
        Args, CheckResult, CommandError, CommandResult,
    },
    http::AttachmentType,
    model::prelude::*,
    prelude::*,
};
use std::borrow::Cow;

#[command]
#[checks(CatCount)]
#[min_args(0)]
#[max_args(1)]
pub fn cat(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let count = args.single::<u8>().unwrap_or(1);
    debug!("Requested {} images", count);

    let data = ctx.data.read();

    let image_path = match data.get::<CatConfig>() {
        Some(config) => &config.image_path,
        None => {
            return Err(CommandError("Failed to get CatConfig".to_string()));
        }
    };

    let images = image_path
        .read_dir()?
        .filter(|file| match file.as_ref().unwrap().path().extension() {
            Some(ext) => match ext.to_str().unwrap().to_lowercase().as_str() {
                "jpg" | "jpeg" => true,
                _ => false,
            },
            None => false,
        })
        .choose_multiple(&mut thread_rng(), count.into())
        .iter()
        .map(|file| file.as_ref().unwrap().path())
        .collect::<Vec<std::path::PathBuf>>();

    debug!("Sending files: {:?}", images);

    let attachments = images
        .iter()
        .map(|image| {
            let mut buffer = Vec::new();
            image::open(image)
                .unwrap()
                .thumbnail(1920, 1920)
                .write_to(&mut buffer, image::ImageOutputFormat::Jpeg(100))
                .unwrap();

            AttachmentType::Bytes {
                data: Cow::from(buffer),
                filename: image.file_name().unwrap().to_str().unwrap().to_string(),
            }
        })
        .collect::<Vec<AttachmentType>>();

    //debug!("Sending file size: {:?}", attachment.iter().sum::<u16>());
    debug!(
        "Attachment(s) size: {:.2?}MB",
        attachments
            .iter()
            .map(|attachment| {
                match attachment {
                    AttachmentType::Bytes { data, filename: _ } => data.len(),
                    _ => 0,
                }
            })
            .sum::<usize>() as f64
            / 1024.0
            / 1024.0
    );

    msg.channel_id
        .send_message(&ctx.http, |m| m.add_files(attachments))?;

    Ok(())
}

#[check]
#[name = "CatCount"]
fn cat_count_check(ctx: &mut Context, _: &Message, args: &mut Args) -> CheckResult {
    // Cat command defaults to 1 if no arg is given so we don't need to check anything
    if args.is_empty() {
        return true.into();
    }

    let data = ctx.data.read();

    match data.get::<CatConfig>() {
        Some(config) => match args.single::<u8>() {
            Ok(count) => {
                if count < 1 {
                    return CheckResult::new_user("Count has to be at least 1");
                }
                if count > config.max_images {
                    return CheckResult::new_user(format!(
                        "Count can be max {}",
                        config.max_images
                    ));
                }
            }
            Err(_) => return CheckResult::new_user("Count has to be positive integer"),
        },
        None => {
            return CheckResult::new_user_and_log("Internal error", "Failed to get CatConfig");
        }
    };

    // Reset the args position so the command can get correct arguemnts
    // No need to reset it for failure states since it won't reach the command anyway
    args.restore();

    true.into()
}
