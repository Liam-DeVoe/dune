//! Port of `Server/GameHub_Chat.cs`. Package H3.

use treachery_engine::enumerations::ErrorType;

use super::{GameHub, HubResult};
use crate::clients::Callback;
use crate::model::chat_message::{GameChatMessage, GlobalChatMessage};
use crate::model::result::{VoidContents, VoidResult};

impl GameHub {
    /// C#: `public async Task<VoidResult> SendChatMessage(string userToken, string gameId, GameChatMessage e)` (GameHub_Chat.cs:5)
    pub async fn send_chat_message(&mut self, user_token: &str, game_id: &str, e: GameChatMessage) -> HubResult<VoidResult> {
        if let Err(error) = self.are_valid::<VoidContents>(Some(user_token), Some(game_id)) {
            return Ok(error);
        }

        self.clients.group(game_id, Callback::HandleChatMessage(e));
        Ok(Self::success())
    }

    /// C#: `public async Task<VoidResult> SendGlobalChatMessage(string userToken, GlobalChatMessage message)` (GameHub_Chat.cs:14)
    pub async fn send_global_chat_message(&mut self, user_token: &str, message: GlobalChatMessage) -> HubResult<VoidResult> {
        if !self.users_by_user_token.contains_key(user_token) {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        }

        self.clients.all(Callback::HandleGlobalChatMessage(message));
        Ok(Self::success())
    }
}
