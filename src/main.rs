use std::fs::{read, write};

use poise::{
    serenity_prelude::{self as serenity, CreateAttachment},
    CreateReply,
};
use serde::Deserialize;
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
        }
        Ok(o) => {
            if !o.status.success() {
                let response = format!(
                    "**Invalid Typst Math Syntax**\n{}",
                    String::from_utf8(o.stderr)
                        .unwrap_or_default()
                        .split("\n")
                        .next()
                        .unwrap_or_default()
                );
                ctx.say(response).await?;
                return Ok(());
            }
        }
    }

    let png = read(&png_path)?;

    ctx.send(CreateReply::default().attachment(CreateAttachment::bytes(png, "rendered.png")))
        .await?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct OWStats {
    general: GeneralStats,
}

#[derive(Deserialize, Debug)]
struct GeneralStats {
    average: AverageStats,
    games_lost: i32,
    games_won: i32,
    kda: f32,
    winrate: f32,
}

#[derive(Deserialize, Debug)]
struct AverageStats {
    damage: f32,
    healing: f32,
}

/// Get Overwatch Stats of a Player
#[poise::command(slash_command, prefix_command)]
async fn owstats(ctx: Context<'_>, #[description = "player"] player: String) -> Result<(), Error> {
    let player = player.replace('#', "-").replace(' ', "");
    let resp = match reqwest::get(format!(
        "https://overfast-api.tekrop.fr/players/{player}/stats/summary"
    ))
    .await
    {
        Err(e) => {
            println!("Request Error: {e:?}");
            return Ok(());
        }
        Ok(v) => v,
    };
    if let Err(e) = resp.error_for_status_ref() {
        println!("Request Status: {:?}", e.status());
        return Ok(());
    }
    let json = match resp.json::<OWStats>().await {
        Err(e) => {
            println!("Json Parse Error: {e:?}");
            return Ok(());
        }
        Ok(v) => v,
    };
    ctx.send(CreateReply::default().content(format!(
        r#"
**STATS FOR PLAYER {}**
📊      **KDA:** {}
📊      **Winrate:** {}%
💣      **Average Damage:** {}
💛      **Average Healing:** {}
📈      **Games Won:** {}
📉      **Games Lost:** {}
        "#,
        player,
        json.general.kda,
        json.general.winrate,
        json.general.average.damage,
        json.general.average.healing,
        json.general.games_won,
        json.general.games_lost,
    )))
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![math(), owstats()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
