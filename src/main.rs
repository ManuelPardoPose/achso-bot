use std::{fs::{read, write}};

use poise::{serenity_prelude::{self as serenity, CreateAttachment, GuildId}, CreateReply};
use tempfile::tempdir;
use tokio::process::Command;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Math rendering via typst
#[poise::command(slash_command, prefix_command)]
async fn math(
    ctx: Context<'_>,
    #[description = "math expression (typst syntax)"] expression: String,
) -> Result<(), Error> {
    let exp_typst = format!(
        r#"
#set page(margin: 0.5cm, width: auto, height: auto, fill: none)
#set text(fill: white, size: 0.7cm)
$ {} $
        "#,
        expression
    );
    let dir = tempdir()?;
    let typ_path = dir.path().join("math.typ");
    let png_path = dir.path().join("math.png");
    write(&typ_path, exp_typst.clone())?;
    let output = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&png_path)
        .output()
        .await;

    match output {
        Err(e) => {
            let response = "An Error occured. Please contact the bot developer.";
            ctx.say(response).await?;
            println!("Fatal Error occured: {e}");
            return Ok(());
        },
        Ok(o) => {
            if !o.status.success() {
                let response = format!("**Invalid Typst Math Syntax**\n{}", String::from_utf8(o.stderr).unwrap_or_default().split("\n").next().unwrap_or_default());
                ctx.say(response).await?;
                return Ok(());
            }
        },
    }

    let png = read(&png_path)?;

    ctx.send(
        CreateReply::default()
            .attachment(CreateAttachment::bytes(png, "rendered.png"))
    ).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![math()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                // poise::builtins::register_globally(ctx, &framework.options().commands).await?; // used for deployment
                poise::builtins::register_in_guild(ctx, &framework.options().commands, GuildId::new(725690997031567421)).await?; // used for dev
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
