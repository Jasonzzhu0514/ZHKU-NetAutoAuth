use super::*;
use zeroize::{Zeroize, Zeroizing};

impl Client {
    pub fn account(&self) -> AccountInput {
        AccountInput {
            username: self.settings.username.clone(),
            password: self.settings.password.to_string(),
            remember_password: self.settings.remember,
            auto_login: self.settings.auto_login,
        }
    }

    pub fn set_account(&mut self, input: AccountInput) -> Result<Option<String>, String> {
        self.set_account_with(input, |settings| {
            storage::path().and_then(|path| storage::save(&path, settings))
        })
    }

    fn set_account_with(
        &mut self,
        mut input: AccountInput,
        save: impl FnOnce(&storage::Settings) -> Result<(), String>,
    ) -> Result<Option<String>, String> {
        if self.receiver.is_some() || self.guard {
            input.password.zeroize();
            return Err("请先停止守护并等待网络请求完成".into());
        }
        if input.username.len() > 256 || input.password.len() > 1024 {
            input.password.zeroize();
            return Err("账号或密码过长，凭据未保存".into());
        }
        let configured = !input.username.trim().is_empty() && !input.password.is_empty();
        let next = storage::Settings {
            username: input.username.trim().into(),
            password: Zeroizing::new(std::mem::take(&mut input.password)),
            remember: input.remember_password || (input.auto_login && configured),
            auto_login: input.auto_login && configured,
        };
        self.settings = next;
        self.mark_changed();
        match save(&self.settings) {
            Ok(()) => {
                self.saved_username =
                    (!self.settings.username.is_empty()).then(|| self.settings.username.clone());
                Ok(None)
            }
            Err(error) => {
                self.log(&error, true);
                Ok(Some(error))
            }
        }
    }

    // Restarting for an update requires the session choices to survive exit.
    pub fn persist_account(&mut self) -> Result<(), String> {
        storage::save(&storage::path()?, &self.settings)?;
        self.saved_username =
            (!self.settings.username.is_empty()).then(|| self.settings.username.clone());
        self.mark_changed();
        Ok(())
    }

    pub fn restore_account(&mut self) -> Result<AccountInput, String> {
        if self.receiver.is_some() || self.guard {
            return Err("请先停止守护并等待网络请求完成".into());
        }
        self.settings = storage::load(&storage::path()?)?;
        self.mark_changed();
        Ok(self.account())
    }

    pub fn forget_account(&mut self) -> Result<(), String> {
        if self.receiver.is_some() || self.guard {
            return Err("请先停止守护并等待网络请求完成".into());
        }
        storage::delete(&storage::path()?)?;
        self.settings.username.zeroize();
        self.settings.password.zeroize();
        self.settings.remember = false;
        self.settings.auto_login = false;
        self.saved_username = None;
        self.guard_pending = false;
        self.log("已删除保存的凭据，并清除本次输入", false);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_save_keeps_session_credentials_and_old_saved_account() {
        let mut client = Client {
            saved_username: Some("previous-test-account".into()),
            ..Client::default()
        };
        let warning = client
            .set_account_with(
                AccountInput {
                    username: "session-test-account".into(),
                    password: "test-only-password".into(),
                    remember_password: false,
                    auto_login: false,
                },
                |_| Err("凭据目录不可写".into()),
            )
            .unwrap();
        assert_eq!(warning.as_deref(), Some("凭据目录不可写"));
        assert_eq!(client.settings.username, "session-test-account");
        assert_eq!(client.settings.password.as_str(), "test-only-password");
        assert_eq!(
            client.saved_username.as_deref(),
            Some("previous-test-account")
        );
        client.set_guard(true).unwrap();
        assert!(client.guard);
        assert!(!client.settings.remember);
        assert!(
            client
                .logs
                .iter()
                .all(|entry| !entry.message.contains("test-only-password"))
        );
    }
}
