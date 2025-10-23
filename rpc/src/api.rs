use crate::handlers::UserHandler;

pub struct RpcServer {
    user_handler: UserHandler,
}

impl RpcServer {
    pub fn new() -> Self {
        Self {
            user_handler: UserHandler::new(),
        }
    }

    pub fn start(&self) {
        println!("RPC Server started");
        // Your RPC server implementation would go here
        // e.g., HTTP server, gRPC, tonic, tarpc, etc.
    }

    pub fn user_handler(&self) -> &UserHandler {
        &self.user_handler
    }

    pub fn user_handler_mut(&mut self) -> &mut UserHandler {
        &mut self.user_handler
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}
