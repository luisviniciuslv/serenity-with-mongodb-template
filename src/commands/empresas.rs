use std::time::Duration;

use poise::serenity_prelude::{
    ButtonStyle, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};
use poise::CreateReply;

use crate::db::{get_user, update_user};
use crate::model::{BusinessModel, UserModel, BUSINESSES};
use crate::{Context, Error};

#[poise::command(slash_command, prefix_command)]
pub async fn empresas(ctx: Context<'_>) -> Result<(), Error> {
    let author = ctx.author().clone();
    let user_id = author.id.to_string();
    let mut current_index: usize = 0;

    let user_db = get_user(&user_id).await?;
    let (embed, components) = build_empresas_view(&user_db, &author.name, author.face(), current_index, None);

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            components: Some(components),
            ..Default::default()
        })
        .await?;

    let message = reply.message().await?;

    loop {
        let interaction = message
            .await_component_interaction(ctx.serenity_context())
            .author_id(author.id)
            .timeout(Duration::from_secs(180))
            .await;

        let Some(interaction) = interaction else {
            break;
        };

        let mut status_message: Option<String> = None;
        let custom_id = interaction.data.custom_id.as_str();

        match custom_id {
            "empresas_prev" => {
                current_index = if current_index == 0 {
                    BUSINESSES.len() - 1
                } else {
                    current_index - 1
                };
            }
            "empresas_next" => {
                current_index = (current_index + 1) % BUSINESSES.len();
            }
            "empresas_buy" => {
                status_message = Some(buy_or_upgrade_business(&user_id, current_index).await?);
            }
            _ => {}
        }

        let user_db = get_user(&user_id).await?;
        let (embed, components) = build_empresas_view(
            &user_db,
            &author.name,
            author.face(),
            current_index,
            status_message,
        );

        interaction
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(components),
                ),
            )
            .await?;
    }

    Ok(())
}

fn build_empresas_view(
    user: &UserModel,
    username: &str,
    avatar_url: String,
    current_index: usize,
    status_message: Option<String>,
) -> (CreateEmbed, Vec<CreateActionRow>) {
    let business = BUSINESSES[current_index];
    let owned = user.businesses.iter().find(|item| item.name == business.name);

    let (action_label, action_price, level_text, own_text) = if let Some(owned_business) = owned {
        (
            "Aumentar level",
            owned_business.price * 2,
            owned_business.level,
            "Sim",
        )
    } else {
        ("Comprar", business.price, 0, "Nao")
    };

    let mut embed = CreateEmbed::new()
        .title("Empresas")
        .description(format!(
            "Empresa {}/{}\nUse os botoes para navegar e comprar/upar.",
            current_index + 1,
            BUSINESSES.len()
        ))
        .field("Nome", business.name, true)
        .field("Sua posse", own_text, true)
        .field("Level atual", level_text.to_string(), true)
        .field("Reward base", format!("{} coin(s)/tick", business.reward), true)
        .field("Preco da acao", action_price.to_string(), true)
        .field("Seu saldo", user.coins.to_string(), true)
        .field("Acao", action_label, false)
        .footer(CreateEmbedFooter::new(format!("Loja de {}", username)).icon_url(avatar_url));

    if let Some(status) = status_message {
        embed = embed.field("Resultado", status, false);
    }

    let action_row = CreateActionRow::Buttons(vec![
        CreateButton::new("empresas_prev")
            .label("Anterior")
            .style(ButtonStyle::Secondary),
        CreateButton::new("empresas_buy")
            .label(action_label)
            .style(ButtonStyle::Success),
        CreateButton::new("empresas_next")
            .label("Proxima")
            .style(ButtonStyle::Secondary),
    ]);

    (embed, vec![action_row])
}

async fn buy_or_upgrade_business(user_id: &str, current_index: usize) -> Result<String, Error> {
    let catalog = BUSINESSES[current_index];
    let mut user = get_user(user_id).await?;

    if let Some(owned_index) = user
        .businesses
        .iter()
        .position(|business| business.name == catalog.name)
    {
        let upgrade_price = user.businesses[owned_index].price * 2;

        if user.coins < upgrade_price {
            return Ok(format!(
                "Saldo insuficiente para upar {}. Necessario: {}",
                catalog.name, upgrade_price
            ));
        }

        user.coins -= upgrade_price;
        user.businesses[owned_index].level += 1;
        user.businesses[owned_index].price = upgrade_price;
        let new_level = user.businesses[owned_index].level;
        update_user(&user).await?;

        Ok(format!(
            "{} upada para level {} por {} coins.",
            catalog.name, new_level, upgrade_price
        ))
    } else {
        if user.coins < catalog.price {
            return Ok(format!(
                "Saldo insuficiente para comprar {}. Necessario: {}",
                catalog.name, catalog.price
            ));
        }

        user.coins -= catalog.price;
        user.businesses.push(BusinessModel {
            name: catalog.name.to_string(),
            level: 1,
            reward: catalog.reward,
            price: catalog.price,
        });
        update_user(&user).await?;

        Ok(format!(
            "{} comprada por {} coins.",
            catalog.name, catalog.price
        ))
    }
}
