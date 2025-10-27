use crate::error::RpcResult;
use pipeline_service::models::User;
use pipeline_service::services::UserService;

pub struct UserHandler {
    service: UserService,
}

impl UserHandler {
    pub fn new() -> Self {
        Self {
            service: UserService::new(),
        }
    }

    pub fn create_user(&mut self, name: String, email: String) -> RpcResult<User> {
        Ok(self.service.create_user(name, email)?)
    }

    pub fn get_user(&self, id: u64) -> RpcResult<User> {
        Ok(self.service.get_user(id)?.clone())
    }

    pub fn list_users(&self) -> RpcResult<Vec<User>> {
        Ok(self.service.list_users().into_iter().cloned().collect())
    }

    pub fn delete_user(&mut self, id: u64) -> RpcResult<()> {
        Ok(self.service.delete_user(id)?)
    }
}

impl Default for UserHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_user() {
        let mut handler = UserHandler::new();
        let user = handler
            .create_user("Alice".to_string(), "alice@example.com".to_string())
            .unwrap();
        
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");

        let fetched_user = handler.get_user(user.id).unwrap();
        assert_eq!(fetched_user.name, "Alice");
    }

    #[test]
    fn test_list_users() {
        let mut handler = UserHandler::new();
        handler
            .create_user("Alice".to_string(), "alice@example.com".to_string())
            .unwrap();
        handler
            .create_user("Bob".to_string(), "bob@example.com".to_string())
            .unwrap();

        let users = handler.list_users().unwrap();
        assert_eq!(users.len(), 2);
    }
}
