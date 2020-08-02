use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    Args, CommandResult,
    macros::command,
};

#[command]
pub fn cat(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let count = args.single::<u8>().unwrap_or(1);

    msg.channel_id.say(&ctx.http, format!("Requested {} images", count))?;

    Ok(())
}
