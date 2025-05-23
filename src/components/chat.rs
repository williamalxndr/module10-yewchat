use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::{User, services::websocket::WebsocketService};
use crate::services::event_bus::EventBus;

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
    React(usize, String),
}

#[derive(Deserialize, PartialEq, Clone)]
pub struct MessageData {
    pub from: String,
    pub message: String,
    pub reactions: Option<Vec<(String, Vec<String>)>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketMessage {
    pub message_type: MsgTypes,
    pub data_array: Option<Vec<String>>,
    pub data: Option<String>,
}

#[derive(Clone)]
pub struct UserProfile {
    pub name: String,
    pub avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    wss: WebsocketService,
    messages: Vec<MessageData>,
    _producer: Box<dyn Bridge<EventBus>>,
    current_user: String,
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");

        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let register_msg = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.clone()),
            data_array: None,
        };

        if let Ok(_) = wss.tx.clone().try_send(serde_json::to_string(&register_msg).unwrap()) {
            log::debug!("Registered user {}", username);
        }

        Chat {
            users: vec![],
            chat_input: NodeRef::default(),
            wss,
            messages: vec![],
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
            current_user: username,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(data) => {
                let msg: WebSocketMessage = serde_json::from_str(&data).unwrap();
                match msg.message_type {
                    MsgTypes::Users => {
                        let usernames = msg.data_array.unwrap_or_default();
                        self.users = usernames
                            .into_iter()
                            .map(|name| UserProfile {
                                avatar: format!("https://avatars.dicebear.com/api/adventurer-neutral/{}.svg", name),
                                name,
                            })
                            .collect();
                        true
                    }
                    MsgTypes::Message => {
                        let message_data: MessageData = serde_json::from_str(&msg.data.unwrap()).unwrap();
                        self.messages.push(message_data);
                        true
                    }
                    _ => false,
                }
            }
            Msg::SubmitMessage => {
                if let Some(input) = self.chat_input.cast::<HtmlInputElement>() {
                    let text = input.value();
                    let message = WebSocketMessage {
                        message_type: MsgTypes::Message,
                        data: Some(text.clone()),
                        data_array: None,
                    };
                    if !text.trim().is_empty() {
                        let _ = self.wss.tx.clone().try_send(serde_json::to_string(&message).unwrap());
                    }
                    input.set_value("");
                }
                false
            }
            Msg::React(index, emoji) => {
                if let Some(msg) = self.messages.get_mut(index) {
                    let user = self.current_user.clone();
                    if let Some(reactions) = &mut msg.reactions {
                        if let Some((_, users)) = reactions.iter_mut().find(|(e, _)| e == &emoji) {
                            if users.contains(&user) {
                                users.retain(|u| u != &user);
                            } else {
                                users.push(user);
                            }
                        } else {
                            reactions.push((emoji, vec![user]));
                        }
                    } else {
                        msg.reactions = Some(vec![(emoji, vec![user])]);
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);
        let react = ctx.link().callback(|(idx, emoji): (usize, String)| Msg::React(idx, emoji));
        let emojis = vec!["👍", "❤️", "😂", "😮", "😢", "👏"];

        html! {
            <div class="flex w-screen">
                <div class="w-56 h-screen bg-gray-100 overflow-auto">
                    <div class="text-xl p-3 font-semibold">{"Users"}</div>
                    {
                        self.users.iter().map(|u| html! {
                            <div class="flex m-3 bg-white rounded-lg p-2">
                                <img class="w-12 h-12 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                                <div class="p-3 text-sm">
                                    <div class="font-medium">{ &u.name }</div>
                                    <div class="text-xs text-gray-400">{"Hi there!"}</div>
                                </div>
                            </div>
                        }).collect::<Html>()
                    }
                </div>
                <div class="flex-1 flex flex-col h-screen">
                    <div class="h-14 border-b p-3 text-xl font-semibold">{"💬 Chat!"}</div>
                    <div class="flex-1 overflow-auto border-b p-4 space-y-4">
                        {
                            self.messages.iter().enumerate().map(|(i, m)| {
                                let fallback = UserProfile {
                                    name: m.from.clone(),
                                    avatar: format!("https://avatars.dicebear.com/api/adventurer-neutral/{}.svg", m.from),
                                };
                                let user_ref: &UserProfile = self.users.iter().find(|u| u.name == m.from).unwrap_or(&fallback);
                                html! {
                                    <div class="flex items-start space-x-3 bg-gray-100 p-3 rounded-xl max-w-lg">
                                        <img class="w-10 h-10 rounded-full" src={user_ref.avatar.clone()} alt="avatar"/>
                                        <div>
                                            <div class="text-sm font-medium">{ &m.from }</div>
                                            <div class="text-base">{ &m.message }</div>
                                            <div class="mt-2 flex flex-wrap gap-1">
                                                {
                                                    emojis.iter().map(|&emoji| {
                                                        let count = m.reactions.as_ref()
                                                            .and_then(|rs| rs.iter().find(|(e, _)| e == emoji))
                                                            .map(|(_, users)| users.len())
                                                            .unwrap_or(0);
                                                        let emoji_cb = emoji.to_string();
                                                        let react_cb = react.clone();
                                                        let onclick = Callback::from(move |_| react_cb.emit((i, emoji_cb.clone())));
                                                        html! {
                                                            <button {onclick} class="flex items-center bg-white px-2 py-1 text-sm rounded-full border hover:bg-gray-200 transition">
                                                                <span>{ emoji }</span>
                                                                {
                                                                    if count > 0 {
                                                                        html! { <span class="ml-1 text-xs font-semibold">{ count }</span> }
                                                                    } else {
                                                                        html! {}
                                                                    }
                                                                }
                                                            </button>
                                                        }
                                                    }).collect::<Html>()
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    <div class="h-16 flex items-center p-4">
                        <input
                            ref={self.chat_input.clone()}
                            type="text"
                            placeholder="Type a message..."
                            class="flex-1 rounded-full bg-gray-100 px-4 py-2 focus:outline-none"
                        />
                        <button onclick={submit} class="ml-2 w-10 h-10 bg-blue-600 rounded-full flex items-center justify-center text-white">
                            <svg class="w-5 h-5 fill-current" viewBox="0 0 24 24"><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"/></svg>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
