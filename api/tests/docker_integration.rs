use std::process::Command;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_nmcli_in_docker() {
    // 1. Build the docker image locally for testing
    println!("Building test image...");
    let build_status = Command::new("docker")
        .current_dir("..")
        .args(&["buildx", "bake", "-f", "docker-bake.hcl", "machine-setup-test"])
        .status()
        .expect("Failed to build docker image");
    
    assert!(build_status.success(), "Docker build failed");

    // 2. Start the container in detached, privileged mode
    println!("Starting test container...");
    let container_id = String::from_utf8(
        Command::new("docker")
            .args(&[
                "run",
                "-d",
                "--privileged",
                "--rm",
                "-e", "MOCK_NETWORK=0", // Use real nmcli
                "machine-setup-test"
            ])
            .output()
            .expect("Failed to start docker container")
            .stdout
    ).unwrap().trim().to_string();

    println!("Started container: {}", container_id);

    // Ensure we clean up the container no matter what happens
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            println!("Cleaning up container {}...", self.0);
            let _ = Command::new("docker").args(&["stop", &self.0]).status();
        }
    }
    let _cleanup = Cleanup(container_id.clone());

    // 3. Wait for NetworkManager and the API to boot
    println!("Waiting for services to boot inside container...");
    sleep(Duration::from_secs(5)).await;

    // 4. Test API via docker exec
    // Let's create a dummy dummy wifi connection to simulate a saved network
    let add_conn_status = Command::new("docker")
        .args(&[
            "exec",
            &container_id,
            "nmcli", "connection", "add", "type", "wifi", "ifname", "wlan0", "con-name", "Test_WiFi", "ssid", "Test_SSID"
        ])
        .status()
        .expect("Failed to execute nmcli add");
    
    assert!(add_conn_status.success(), "Failed to add dummy nmcli connection");

    // 5. Query the API (which is running inside the container on port 3000)
    // We'll just use curl from inside the container to test the API directly
    println!("Testing GET /api/wifi/saved...");
    let saved_req = Command::new("docker")
        .args(&[
            "exec",
            &container_id,
            "curl", "-s", "http://127.0.0.1:3000/api/wifi/saved"
        ])
        .output()
        .expect("Failed to query API");
        
    assert!(saved_req.status.success(), "Failed to query API");
    
    let saved_json = String::from_utf8_lossy(&saved_req.stdout);
    println!("Saved networks: {}", saved_json);
    
    // Check if the response contains our "Test_WiFi"
    assert!(saved_json.contains("Test_WiFi"), "Saved networks didn't contain the test SSID: {}", saved_json);
    
    println!("Integration test passed!");
}
