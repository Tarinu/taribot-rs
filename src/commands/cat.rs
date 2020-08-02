use crate::CatConfig;
use serenity::framework::standard::{
    macros::{check, command},
    Args, CheckResult, CommandResult,
};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[checks(CatCount)]
#[min_args(0)]
#[max_args(1)]
pub fn cat(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let count = args.single::<u8>().unwrap_or(1);

    msg.channel_id
        .say(&ctx.http, format!("Requested {} images", count))?;

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

    true.into()
}
