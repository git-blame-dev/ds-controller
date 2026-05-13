pub mod backend;
pub mod mapping;
pub mod protocol;
pub mod receiver;

#[cfg(test)]
mod tests {
    use crate::{backend, mapping, protocol, receiver};

    #[test]
    fn exports_receiver_domain_modules() {
        let _ = std::any::type_name::<dyn backend::ControllerBackend>();
        let _ = std::any::type_name::<mapping::ControllerOutputState>();
        let _ = std::any::type_name::<protocol::ControllerState>();
        let _ = std::any::type_name::<receiver::ReceiverConfig>();
    }
}
