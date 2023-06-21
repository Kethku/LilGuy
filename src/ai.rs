use anyhow::{anyhow, Result};
use openai::{
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
    set_key,
};
use serde::{Deserialize, Serialize};
use serenity::{model::prelude::Message, prelude::Context};
use std::env;

pub fn auth() {
    set_key(env::var("OPENAI_KEY").unwrap());
}

#[derive(Serialize, Deserialize)]
pub struct Bot {
    name: String,
    identity: String,
    rules: Vec<String>,
    examples: Vec<(String, String)>,
    previous_messages: Vec<ChatCompletionMessage>,
}

impl Bot {
    pub fn open(name: &str) -> Self {
        let file = format!("{}.json", name);
        std::fs::read_to_string(file)
            .ok()
            .and_then(|history| serde_json::from_str(&history).ok())
            .unwrap_or(Self {
            name: name.to_string(),
            identity: "You are a cute little guy named Astro that tries to be helpful but doesn't really know much because they are just a little guy.".to_string(),
            rules: Vec::new(),
            examples: Vec::new(),
            previous_messages: Vec::new(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let file = format!("{}.json", self.name);
        std::fs::write(file, serde_json::to_string(&self)?)
            .map_err(|e| anyhow!("Failed to write state file: {}", e))
    }

    pub fn set_identity(&mut self, identity: &str) {
        self.identity = identity.to_string();
    }

    pub fn with_identity(mut self, identity: &str) -> Self {
        self.set_identity(identity);
        self
    }

    pub fn with_rule(mut self, rule: &str) -> Self {
        self.rules.push(rule.to_string());
        self
    }

    pub fn reset(mut self) -> Self {
        self.set_identity("You are a cute little guy named Astro that tries to be helpful but doesn't really know much because they are just a little guy.");
        self.rules.clear();
        self.examples.clear();
        self.previous_messages.clear();
        self
    }

    pub async fn respond(&mut self, query: Option<&str>) -> Result<String> {
        if let Some(query) = query {
            self.previous_messages.push(ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: query.to_string(),
                name: None,
            });
        }

        let system = format!("{}\n{}", self.identity, self.rules.join("\n"),);

        let mut messages = vec![ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: system,
            name: Some(self.name.clone()),
        }];

        // Add previous messages
        messages.append(&mut self.previous_messages.clone());

        // Make request
        let completion = ChatCompletion::builder("gpt-3.5-turbo", messages)
            .create()
            .await
            .unwrap()
            .unwrap();

        // Add new bot message to previous messages
        let returned_message = completion.choices.first().unwrap().message.clone();

        self.previous_messages.push(returned_message.clone());

        while self.previous_messages.len() > 20 {
            self.previous_messages.remove(0);
        }

        Ok(returned_message.content)
    }
}

pub async fn generate_response(name: &'static str, query: Option<&str>) -> Result<String> {
    let mut bot = Bot::open(name);

    let response = bot.respond(query).await?;

    bot.save()?;

    Ok(response)
}

pub async fn astro(query: &str) -> Result<String> {
    generate_response("astro", Some(query)).await
}

pub async fn greeter() -> Result<String> {
    generate_response("greeter", None).await
}

pub async fn emoji(query: &str) -> Result<String> {
    let mut bot = Bot::open("emoji");
    let response = bot.respond(Some(query)).await?;
    bot.save()?;
    Ok(response)
}

pub async fn directed_at_bot(ctx: Context, last_5_messages: Vec<Message>) -> Result<bool> {
    // Convert discord messages to a concatenated chat log
    let mut log = String::new();
    for message in last_5_messages {
        let mut author = message
            .author_nick(&ctx)
            .await
            .unwrap_or(message.author.name);

        if author == "DM" {
            author = "Astro".to_string();
        }

        log.push_str(&format!("{}: {}\n", author, message.content.clone()));
    }
    let mut bot = Bot::open("directed_at_bot");
    let response = bot.respond(Some(&log)).await?;
    Ok(response.as_str() == "TRUE")
}
