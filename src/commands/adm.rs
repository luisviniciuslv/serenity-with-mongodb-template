use poise::serenity_prelude::{Mentionable, User};
use poise::CreateReply;

use crate::db::{clear_users_collection, update_coins};
use crate::{Context, Error};

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn add_coins(
    ctx: Context<'_>,
    #[description = "Usuario que vai receber/remover coins"] user: User,
    #[description = "Quantidade de coins (use negativo para remover)"] coins: i64,
) -> Result<(), Error> {
    let updated_user = update_coins(&user.id.to_string(), coins).await?;

    ctx.send(CreateReply {
        content: Some(format!(
            "Coins atualizados para {}. Saldo atual: {}",
            user.mention(),
            updated_user.coins
        )),
        ..Default::default()
    })
    .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "ADMINISTRATOR",
    rename = "clear_db"
)]
pub async fn clear_db(
    ctx: Context<'_>,
    #[description = "Digite CONFIRMAR para limpar todos os usuarios"] confirmacao: String,
) -> Result<(), Error> {
    if confirmacao != "CONFIRMAR" {
        ctx.send(CreateReply {
            content: Some(
                "Operacao cancelada. Para limpar o banco use exatamente: CONFIRMAR".to_string(),
            ),
            ..Default::default()
        })
        .await?;
        return Ok(());
    }

    let deleted_count = clear_users_collection().await?;

    ctx.send(CreateReply {
        content: Some(format!(
            "Banco limpo com sucesso. Usuarios removidos: {}",
            deleted_count
        )),
        ..Default::default()
    })
    .await?;

    Ok(())
}
