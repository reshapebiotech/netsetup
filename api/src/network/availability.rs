use super::nmcli::parse_nmcli_terse_line;
use super::transaction::Commands;
use super::{NetworkError, Result};
use uuid::Uuid;

/// Inspect saved configuration, including disconnected profiles, without probing the network.
pub(super) async fn has_client_configuration(commands: &impl Commands) -> Result<bool> {
    let args = ["-t", "-f", "UUID,TYPE", "connection", "show"].map(str::to_owned);
    let raw = commands.run("nmcli", &args).await?;
    for line in raw.lines().filter(|line| !line.is_empty()) {
        let fields = parse_nmcli_terse_line(line);
        if fields.len() != 2 || Uuid::parse_str(&fields[0]).is_err() {
            return Err(NetworkError::Internal(
                "Could not interpret saved network profiles".into(),
            ));
        }
        match fields[1].as_str() {
            "802-3-ethernet" => return Ok(true),
            "802-11-wireless" => {
                let args = [
                    "-g",
                    "802-11-wireless.mode",
                    "connection",
                    "show",
                    "uuid",
                    &fields[0],
                ]
                .map(str::to_owned);
                let mode = commands.run("nmcli", &args).await?;
                if mode.trim() != "ap" {
                    return Ok(true);
                }
            }
            _ => {}
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct Script(Mutex<VecDeque<Result<String>>>);

    #[async_trait]
    impl Commands for Script {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            assert_eq!(program, "nmcli");
            assert!(args.iter().any(|arg| arg == "show"));
            assert!(!args.iter().any(|arg| arg == "--show-secrets"));
            self.0
                .lock()
                .unwrap()
                .pop_front()
                .expect("Unexpected query")
        }
    }

    fn script(outputs: &[&str]) -> Script {
        Script(Mutex::new(
            outputs.iter().map(|s| Ok((*s).into())).collect(),
        ))
    }

    #[tokio::test]
    async fn empty_profiles_or_only_access_points_are_unconfigured() {
        assert!(!has_client_configuration(&script(&[""])).await.unwrap());
        let profiles = "11111111-1111-1111-1111-111111111111:802-11-wireless\n22222222-2222-2222-2222-222222222222:bridge";
        assert!(
            !has_client_configuration(&script(&[profiles, "ap"]))
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn disconnected_client_and_wired_profiles_prevent_fallback() {
        let wifi = "11111111-1111-1111-1111-111111111111:802-11-wireless";
        for mode in ["infrastructure", "", "mesh", "adhoc"] {
            assert!(
                has_client_configuration(&script(&[wifi, mode]))
                    .await
                    .unwrap()
            );
        }
        let wired = "11111111-1111-1111-1111-111111111111:802-3-ethernet";
        assert!(has_client_configuration(&script(&[wired])).await.unwrap());
    }

    #[tokio::test]
    async fn query_failure_or_malformed_output_is_not_evidence_of_no_configuration() {
        assert!(
            has_client_configuration(&script(&["invalid output"]))
                .await
                .is_err()
        );
        let failed = Script(Mutex::new(VecDeque::from([Err(NetworkError::Internal(
            "NetworkManager unavailable".into(),
        ))])));
        assert!(has_client_configuration(&failed).await.is_err());
        let disappeared = Script(Mutex::new(VecDeque::from([
            Ok("11111111-1111-1111-1111-111111111111:802-11-wireless".into()),
            Err(NetworkError::NotFound("Profile disappeared".into())),
        ])));
        assert!(has_client_configuration(&disappeared).await.is_err());
    }
}
