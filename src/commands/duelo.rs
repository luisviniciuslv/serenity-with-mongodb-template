use std::time::Duration;

use poise::serenity_prelude::{Mentionable, ReactionType, User};
use poise::CreateReply;

use crate::db::{get_user, update_coins};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn duelo(
    ctx: Context<'_>,
    #[description = "Membro desafiado"] alvo: User,
    #[description = "Valor da aposta"] aposta: i64,
) -> Result<(), Error> {
    let desafiante = ctx.author().clone();

    if aposta <= 0 {
        ctx.say("Aposta inválida.").await?;
        return Ok(());
    }

    if alvo.id == desafiante.id {
        ctx.say("Você não pode duelar com você mesmo.").await?;
        return Ok(());
    }

    if alvo.bot {
        ctx.say("Não é possível duelar com bots.").await?;
        return Ok(());
    }

    let desafiante_user = get_user(&desafiante.id.to_string()).await?;
    let alvo_user = get_user(&alvo.id.to_string()).await?;

    if desafiante_user.coins < aposta {
        ctx.say("Você não tem coins suficientes para essa aposta.").await?;
        return Ok(());
    }

    if alvo_user.coins < aposta {
        ctx.say("O membro desafiado não tem coins suficientes para essa aposta.").await?;
        return Ok(());
    }

    let reply = ctx
        .send(CreateReply {
            content: Some(format!(
                "{} desafiou {} para um duelo de **{}** coin(s)!\n{} clique no emoji ✅ em até **10 segundos** para aceitar.",
                desafiante.mention(),
                alvo.mention(),
                aposta,
                alvo.mention()
            )),
            ..Default::default()
        })
        .await?;

    let message = reply.message().await?.into_owned();
    message.react(ctx.serenity_context(), '✅').await?;

    let accepted_reaction = message
        .await_reaction(ctx.serenity_context())
        .author_id(alvo.id)
        .timeout(Duration::from_secs(10))
        .await;

    let accepted = matches!(
        accepted_reaction,
        Some(reaction) if matches!(reaction.emoji, ReactionType::Unicode(ref emoji) if emoji == "✅")
    );

    if !accepted {
        message
            .reply(
                ctx.serenity_context(),
                format!(
                    "Duelo cancelado: {} não aceitou a tempo.",
                    alvo.mention()
                ),
            )
            .await?;
        return Ok(());
    }

    // Revalida saldos no momento da execução para evitar corrida com outros jogos.
    let desafiante_user = get_user(&desafiante.id.to_string()).await?;
    let alvo_user = get_user(&alvo.id.to_string()).await?;

    if desafiante_user.coins < aposta || alvo_user.coins < aposta {
        message
            .reply(
                ctx.serenity_context(),
                "Duelo cancelado: um dos jogadores não tem mais saldo suficiente.",
            )
            .await?;
        return Ok(());
    }

    let winner_is_challenger: bool = rand::random();
    let winner = if winner_is_challenger { &desafiante } else { &alvo };
    let loser = if winner_is_challenger { &alvo } else { &desafiante };

    update_coins(&desafiante.id.to_string(), -aposta).await?;
    update_coins(&alvo.id.to_string(), -aposta).await?;
    update_coins(&winner.id.to_string(), aposta * 2).await?;
    let desafiante_final = get_user(&desafiante.id.to_string()).await?;
    let alvo_final = get_user(&alvo.id.to_string()).await?;
    let vencedor_final = get_user(&winner.id.to_string()).await?;

    message
        .reply(
            ctx.serenity_context(),
            format!(
                "Duelo iniciado!\nVencedor: {}\nPerdedor: {}\nPrêmio: {} coin(s)\nSaldo final do vencedor: {} coin(s)\nSaldo final do desafiante: {} coin(s)\nSaldo final do alvo: {} coin(s)",
                winner.mention(),
                loser.mention(),
                aposta * 2,
                vencedor_final.coins,
                desafiante_final.coins,
                alvo_final.coins
            ),
        )
        .await?;

    Ok(())
}
