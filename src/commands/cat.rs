use rand::{seq::IteratorRandom, thread_rng};
use serenity::{
    framework::standard::{
        macros::{check, command},
        Args, CommandResult, Reason,
    },
    model::prelude::*,
    prelude::*,
};
use std::{borrow::Cow, env, io::Cursor, path::PathBuf};
use tracing::{debug, warn};

pub struct CatConfig {
    max_images: u8,
    image_path: PathBuf,
}

impl CatConfig {
    pub fn new() -> Self {
        let mut cat_count = 1;
        match env::var("CAT_MAX_IMAGES") {
            Ok(count) => {
                cat_count = count.parse::<u8>().unwrap();
            }
            Err(_) => {
                warn!("CAT_MAX_IMAGES env not found, defaulting to {}", cat_count);
            }
        }
        debug!("Cat count set to: {}", cat_count);

        let cat_path = env::var("CAT_IMAGE_PATH").expect("CAT_IMAGE_PATH has to be set in env");
        debug!("Cat image path set to: {}", cat_path);
        let path = PathBuf::from(&cat_path);

        if !path.exists() {
            panic!("Given path ({}) doesn't exist", cat_path);
        }
        if !path.is_dir() {
            panic!("Given path ({}) is not directory", cat_path);
        }

        CatConfig {
            max_images: cat_count,
            image_path: path,
        }
    }
}

impl TypeMapKey for CatConfig {
    type Value = CatConfig;
}

#[command]
#[checks(CatCount)]
#[min_args(0)]
#[max_args(1)]
pub async fn cat(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let count = args.single::<u8>().unwrap_or(1);
    debug!("Requested {} images", count);

    let data = ctx.data.read().await;

    let config = data.get::<CatConfig>().ok_or("Failed to get Cat config")?;
    let image_path = &config.image_path;

    let images = image_path
        .read_dir()?
        .filter(|file| match file.as_ref().unwrap().path().extension() {
            Some(ext) => matches!(
                ext.to_str().unwrap().to_lowercase().as_str(),
                "jpg" | "jpeg"
            ),
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
            let mut buffer = Cursor::new(Vec::new());
            image::open(image)
                .unwrap()
                .thumbnail(1920, 1920)
                .write_to(&mut buffer, image::ImageOutputFormat::Jpeg(100))
                .unwrap();

            AttachmentType::Bytes {
                data: Cow::from(buffer.into_inner()),
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
        .send_message(&ctx.http, |m| m.add_files(attachments))
        .await?;

    Ok(())
}

#[check]
#[name = "CatCount"]
async fn cat_count_check(ctx: &Context, _: &Message, args: &mut Args) -> Result<(), Reason> {
    // Cat command defaults to 1 if no arg is given so we don't need to check anything
    if args.is_empty() {
        return Ok(());
    }

    let data = ctx.data.read().await;

    match data.get::<CatConfig>() {
        Some(config) => match args.single::<u8>() {
            Ok(count) => {
                if count < 1 {
                    return Err(Reason::User("Count has to be at least 1".to_owned()));
                }
                if count > config.max_images {
                    return Err(Reason::User(format!(
                        "Count can be max {}",
                        config.max_images
                    )));
                }
            }
            Err(_) => return Err(Reason::User("Count has to be positive integer".to_owned())),
        },
        None => {
            return Err(Reason::UserAndLog {
                user: "Internal error".to_owned(),
                log: "Failed to get CatConfig".to_owned(),
            });
        }
    };

    // Reset the args position so the command can get correct arguemnts
    // No need to reset it for failure states since it won't reach the command anyway
    args.restore();

    Ok(())
}
