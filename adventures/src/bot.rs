use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BotIncommingMessage {
    r#type: String,
    id: String,
    timestamp: String,
    service_url: String,
    channel_id: String,
    from: Option<BotExtMessage>,
    conversation: Option<BotExtMessage>,
    recipient: Option<BotExtMessage>,
    text: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BotExtMessage {
    id: String,
    name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct BotReplyMessage {
    r#type: String,
    from: Option<BotExtMessage>,
    conversation: Option<BotExtMessage>,
    recipient: Option<BotExtMessage>,
    text: String,
    reply_to_id: String,
}

pub async fn bot_reply(incomming_message: &BotIncommingMessage, access_token: &str) -> Result<()> {
    let reply_message = BotReplyMessage {
        r#type: "message".to_string(),
        from: incomming_message.recipient.clone(),
        conversation: incomming_message.conversation.clone(),
        recipient: incomming_message.from.clone(),
        text: "hello from bot".to_string(),
        reply_to_id: incomming_message.id.clone(),
    };

    let client = reqwest::Client::new();
    let data = client
        .post(format!(
            "{}v3/conversations/{}/activities/{}",
            incomming_message.service_url.clone(),
            incomming_message
                .clone()
                .conversation
                .expect("WRONG CONVERSATION")
                .id,
            incomming_message.id
        ))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .json::<BotReplyMessage>(&reply_message)
        .send()
        .await?
        .text()
        .await?;
    log::error!("RESP {data:?}");

    Ok(())
}
