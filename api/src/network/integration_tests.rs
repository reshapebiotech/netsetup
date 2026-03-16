#[cfg(test)]
mod tests {
    use std::time::Duration;
    use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};
    use testcontainers::core::{IntoContainerPort, ExecCommand};

    #[tokio::test]
    async fn test_nmcli_in_docker() {
        // We'll use a standard debian image with network-manager installed
        // However, standard testcontainers might struggle with privileged mode and dbus.
        // Let's create a custom image definition.
        
        let container = GenericImage::new("debian", "bookworm-slim")
            .with_wait_for(WaitFor::message_on_stdout("NetworkManager is running"))
            .with_env_var("DEBIAN_FRONTEND", "noninteractive")
            // We can't easily build dockerfiles inline with generic testcontainers
            // Let's write a shell script to run inside it.
            ;
            
        // We will execute a test using docker CLI directly to get full control over --privileged
    }
}
