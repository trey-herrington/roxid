use crate::error::{ServiceError, ServiceResult};
use crate::models::User;
use std::collections::HashMap;

pub struct UserService {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create_user(&mut self, name: String, email: String) -> ServiceResult<User> {
        if name.is_empty() {
            return Err(ServiceError::InvalidInput(
                "Name cannot be empty".to_string(),
            ));
        }

        let user = User::new(self.next_id, name, email);
        self.users.insert(self.next_id, user.clone());
        self.next_id += 1;
        Ok(user)
    }

    pub fn get_user(&self, id: u64) -> ServiceResult<&User> {
        self.users
            .get(&id)
            .ok_or_else(|| ServiceError::NotFound(format!("User with id {} not found", id)))
    }

    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }

    pub fn delete_user(&mut self, id: u64) -> ServiceResult<()> {
        self.users
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| ServiceError::NotFound(format!("User with id {} not found", id)))
    }
}

impl Default for UserService {
    fn default() -> Self {
        Self::new()
    }
}
