//! Test-only helpers shared by `#[cfg(test)]` modules.

/// Whether the Docker daemon that testcontainers would talk to is reachable.
///
/// Letting postgres/mysql tests early-return here keeps the test suite green
/// on machines without a live Docker daemon (including the nix build sandbox),
/// while still running the container-backed tests whenever Docker is present.
pub async fn docker_available() -> bool {
  let docker = match testcontainers_modules::testcontainers::core::client::docker_client_instance()
    .await
  {
    Ok(docker) => docker,
    Err(error) => {
      tracing::debug!(%error, "docker client unavailable");
      return false;
    }
  };
  if docker.ping().await.is_ok() {
    true
  } else {
    tracing::debug!("docker daemon unreachable");
    false
  }
}
