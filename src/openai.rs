use serde::{Deserialize, Serialize};
use worker::{console_log, wasm_bindgen::JsValue, Headers, Request, RequestInit};

// openai chat api
pub async fn call_chat_api(
    msgs: &Vec<Message>,
    key: String,
    endpoint: Option<String>,
) -> Result<String, worker::Error> {
    let mut headers = Headers::new();
    headers.set("Authorization", format!("Bearer {}", key).as_str())?;
    headers.set("api-key", key.as_str())?;
    headers.set("Content-Type", "application/json")?;
    let body = ChatRequest {
        model: "gpt-3.5-turbo-0301".to_string(),
        messages: msgs.clone(),
    };
    console_log!("{:?}", body);
    let req = Request::new_with_init(
        endpoint
            .unwrap_or("https://api.openai.com/v1/chat/completions".to_string())
            .as_str(),
        RequestInit::new()
            .with_method(worker::Method::Post)
            .with_headers(headers)
            .with_body(Some(JsValue::from_str(&serde_json::to_string(&body)?))),
    )?;
    console_log!("{:?}", &req.headers());
    let mut resp = worker::Fetch::Request(req).send().await?;
    let resp_text = resp.text().await?;
    console_log!("{}", &resp_text);
    match serde_json::from_str::<ChatResponse>(&resp_text) {
        Ok(msgs) => Ok(msgs.choices[0].message.content.to_string()),
        Err(_) => {
            let err_resp = serde_json::from_str::<ErrorResponse>(&resp_text)?;
            Err(worker::Error::from(err_resp.error.message))
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    error: Error,
}

#[derive(Serialize, Deserialize)]
pub struct Error {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<serde_json::Value>,
    code: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        Message {
            role: role.to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatResponse {
    id: String,
    object: String,
    created: i64,
    model: String,
    usage: Usage,
    choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    message: Message,
    finish_reason: Option<String>,
    index: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}
