use anyhow::{Context, Result, bail};
use axum::serve::Listener;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::{ServerConfig, version};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

#[derive(Clone)]
pub struct TlsServerConfig {
    acceptor: TlsAcceptor,
}

impl TlsServerConfig {
    pub fn from_paths(
        cert_path: Option<PathBuf>,
        key_path: Option<PathBuf>,
    ) -> Result<Option<Self>> {
        match (cert_path, key_path) {
            (None, None) => Ok(None),
            (Some(_), None) | (None, Some(_)) => {
                bail!("MUSICLIB_TLS_CERT and MUSICLIB_TLS_KEY must be set together")
            }
            (Some(cert_path), Some(key_path)) => Self::load_paths(cert_path, key_path).map(Some),
        }
    }

    fn load_paths(cert_path: PathBuf, key_path: PathBuf) -> Result<Self> {
        let certs = CertificateDer::pem_file_iter(&cert_path)
            .with_context(|| format!("failed to open TLS certificate {}", cert_path.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("failed to read TLS certificate {}", cert_path.display()))?;
        if certs.is_empty() {
            bail!("TLS certificate file contains no certificates");
        }
        let key = PrivateKeyDer::from_pem_file(&key_path)
            .with_context(|| format!("failed to read TLS private key {}", key_path.display()))?;
        let mut config = ServerConfig::builder_with_protocol_versions(&[&version::TLS13])
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("failed to build TLS server config")?;
        // Only advertise protocols this listener path has been verified to serve.
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self {
            acceptor: TlsAcceptor::from(Arc::new(config)),
        })
    }

    pub fn listener(&self, listener: TcpListener) -> TlsListener {
        TlsListener {
            listener,
            acceptor: self.acceptor.clone(),
        }
    }
}

pub struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(value) => value,
                Err(err) => {
                    tracing::error!(error = %err, "TLS TCP accept failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            if let Err(err) = stream.set_nodelay(true) {
                tracing::debug!(%addr, error = %err, "failed to set TCP_NODELAY");
            }
            match self.acceptor.accept(stream).await {
                Ok(stream) => return (stream, addr),
                Err(err) => {
                    tracing::warn!(%addr, error = %err, "TLS handshake failed");
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}
