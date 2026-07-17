mod db;
mod model;

use std::collections::{HashMap, HashSet};
use std::env;

use poise::serenity_prelude::*;
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data;

pub mod commands;
pub mod events;

#[tokio::main]
async fn main() {
    db::init().await.expect("Failed to connect to database");

    dotenv::dotenv().expect("Failed to load .env file");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client = Client::builder(token, intents_config())
        .framework(framework_config())
        .await
        .unwrap();

    if let Err(motivo) = client.start().await {
        println!("Conexão não sucedida\nMotivo: {motivo:?}");
    }
}

fn intents_config() -> GatewayIntents {
    let intents = GatewayIntents::all();

    intents
}

fn framework_config() -> Framework<Data, Error> {
    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".into()),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(events::handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                if ready.guilds.is_empty() {
                    cleanup_global_command_duplicates(ctx).await?;
                    register_missing_global_commands(ctx, &framework.options().commands).await?;
                } else {
                    for guild_status in &ready.guilds {
                        cleanup_guild_command_duplicates(ctx, guild_status.id).await?;
                        register_missing_guild_commands(
                            ctx,
                            &framework.options().commands,
                            guild_status.id,
                        )
                        .await?;
                    }
                }

                Ok(Data {})
            })
        })
        .build();

    framework
}

async fn cleanup_global_command_duplicates(
    ctx: &poise::serenity_prelude::Context,
) -> Result<(), Error> {
    let existing_commands: Vec<poise::serenity_prelude::Command> =
        poise::serenity_prelude::Command::get_global_commands(ctx).await?;
    let duplicate_ids = find_duplicate_command_ids(&existing_commands);

    for command_id in duplicate_ids {
        poise::serenity_prelude::Command::delete_global_command(ctx, command_id).await?;
    }

    Ok(())
}

async fn cleanup_guild_command_duplicates(
    ctx: &poise::serenity_prelude::Context,
    guild_id: GuildId,
) -> Result<(), Error> {
    let existing_commands: Vec<poise::serenity_prelude::Command> =
        guild_id.get_commands(ctx).await?;
    let duplicate_ids = find_duplicate_command_ids(&existing_commands);

    for command_id in duplicate_ids {
        guild_id.delete_command(ctx, command_id).await?;
    }

    Ok(())
}

fn find_duplicate_command_ids(commands: &[poise::serenity_prelude::Command]) -> Vec<CommandId> {
    let mut seen_names: HashMap<&str, CommandId> = HashMap::new();
    let mut duplicate_ids = Vec::new();

    for command in commands {
        if seen_names.contains_key(command.name.as_str()) {
            duplicate_ids.push(command.id);
        } else {
            seen_names.insert(command.name.as_str(), command.id);
        }
    }

    duplicate_ids
}

async fn register_missing_global_commands(
    ctx: &poise::serenity_prelude::Context,
    commands: &[poise::Command<Data, Error>],
) -> Result<(), Error> {
    let existing_commands: Vec<poise::serenity_prelude::Command> =
        poise::serenity_prelude::Command::get_global_commands(ctx).await?;
    let existing_names: HashSet<&str> = existing_commands
        .iter()
        .map(|command| command.name.as_str())
        .collect();

    for command in commands {
        if let Some(slash_command) = command.create_as_slash_command() {
            if !existing_names.contains(command.name.as_str()) {
                poise::serenity_prelude::Command::create_global_command(ctx, slash_command).await?;
            }
        }
    }

    Ok(())
}

async fn register_missing_guild_commands(
    ctx: &poise::serenity_prelude::Context,
    commands: &[poise::Command<Data, Error>],
    guild_id: GuildId,
) -> Result<(), Error> {
    let existing_commands: Vec<poise::serenity_prelude::Command> =
        guild_id.get_commands(ctx).await?;
    let existing_names: HashSet<&str> = existing_commands
        .iter()
        .map(|command| command.name.as_str())
        .collect();

    for command in commands {
        if let Some(slash_command) = command.create_as_slash_command() {
            if !existing_names.contains(command.name.as_str()) {
                guild_id.create_command(ctx, slash_command).await?;
            }
        }
    }

    Ok(())
}
