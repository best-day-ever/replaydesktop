use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use thiserror::Error;
use tokio::net::UdpSocket;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    admission::{AdmissionBackend, AdmissionError, AdmissionRequest},
    store::{Store, StoreError},
    ticket::{TicketError, TicketIssuer},
};

pub const KNOCK_PACKET_BYTES: usize = 52;
const KNOCK_MAGIC: &[u8; 4] = b"RDK1";

#[derive(Clone)]
pub struct KnockService {
    store: Store,
    admission: Arc<dyn AdmissionBackend>,
    ticket_issuer: TicketIssuer,
}

#[derive(Debug, Error)]
pub enum KnockError {
    #[error("invalid knock packet")]
    InvalidPacket,
    #[error("only IPv4 peers are supported")]
    Ipv6Unsupported,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Admission(#[from] AdmissionError),
    #[error(transparent)]
    Ticket(#[from] TicketError),
    #[error("UDP receive failed")]
    Io(#[from] std::io::Error),
}

impl KnockService {
    pub fn new(
        store: Store,
        admission: Arc<dyn AdmissionBackend>,
        ticket_issuer: TicketIssuer,
    ) -> Self {
        Self {
            store,
            admission,
            ticket_issuer,
        }
    }

    pub async fn process_packet(
        &self,
        packet: &[u8],
        source_ipv4: Ipv4Addr,
        now: i64,
    ) -> Result<bool, KnockError> {
        let (session_id, secret) = parse_packet(packet)?;
        let Some(authorization) = self.store.authorize_knock(session_id, &secret, now).await?
        else {
            return Ok(false);
        };
        let request = AdmissionRequest {
            session_id,
            source_ipv4,
            workstation: authorization.workstation.clone(),
            expires_at: authorization.lease_expires_at,
        };
        let lease_id = match self.admission.open(&request).await {
            Ok(lease_id) => lease_id,
            Err(error) => {
                self.store
                    .mark_connection_failed(session_id, "gateway admission failed")
                    .await?;
                return Err(KnockError::Admission(error));
            }
        };
        if let Err(error) = self
            .store
            .record_admission_lease(session_id, source_ipv4, &lease_id)
            .await
        {
            let _close_result = self.admission.close(&lease_id).await;
            return Err(KnockError::Store(error));
        }
        let ticket = match self.ticket_issuer.issue(
            authorization.user_id,
            authorization.workstation.id,
            session_id,
            source_ipv4,
            now,
        ) {
            Ok(ticket) => ticket,
            Err(error) => {
                let _close_result = self.admission.close(&lease_id).await;
                self.store
                    .mark_connection_failed(session_id, "ticket signing failed")
                    .await?;
                return Err(KnockError::Ticket(error));
            }
        };
        if let Err(error) = self
            .store
            .mark_connection_ready(
                session_id,
                source_ipv4,
                &lease_id,
                ticket.jti,
                &ticket.token,
                ticket.issued_at,
                ticket.expires_at,
            )
            .await
        {
            let _close_result = self.admission.close(&lease_id).await;
            return Err(KnockError::Store(error));
        }
        info!(
            %session_id,
            %source_ipv4,
            workstation_id = %authorization.workstation.id,
            "direct admission lease opened"
        );
        Ok(true)
    }

    pub async fn close_for_user(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        now: i64,
    ) -> Result<(), KnockError> {
        if let Some(lease_id) = self.store.close_connection(session_id, user_id).await? {
            self.admission.close(&lease_id).await?;
        }
        self.store.mark_closed(session_id, now).await?;
        Ok(())
    }

    pub async fn expire_leases(&self, now: i64) -> Result<usize, KnockError> {
        let leases = self.store.expired_leases(now).await?;
        let mut closed = 0;
        for lease in leases {
            match self.admission.close(&lease.lease_id).await {
                Ok(()) => {
                    self.store.mark_expired(lease.session_id, now).await?;
                    closed += 1;
                }
                Err(error) => {
                    warn!(
                        session_id = %lease.session_id,
                        %error,
                        "could not close expired admission lease"
                    );
                }
            }
        }
        for lease in self.store.revoked_leases().await? {
            match self.admission.close(&lease.lease_id).await {
                Ok(()) => {
                    self.store.mark_expired(lease.session_id, now).await?;
                    closed += 1;
                }
                Err(error) => {
                    warn!(
                        session_id = %lease.session_id,
                        %error,
                        "could not close revoked admission lease"
                    );
                }
            }
        }
        for lease in self.store.stale_opening_leases(now).await? {
            match self.admission.close(&lease.lease_id).await {
                Ok(()) => {
                    self.store
                        .mark_connection_failed(
                            lease.session_id,
                            "admission setup did not complete before timeout",
                        )
                        .await?;
                    closed += 1;
                }
                Err(error) => {
                    warn!(
                        session_id = %lease.session_id,
                        %error,
                        "could not close interrupted admission lease"
                    );
                }
            }
        }
        Ok(closed)
    }

    pub async fn reconcile_orphaned_leases(&self) -> Result<usize, KnockError> {
        let leases = self.store.orphaned_leases().await?;
        let mut closed = 0;
        for lease in leases {
            self.admission.close(&lease.lease_id).await?;
            self.store
                .mark_connection_failed(lease.session_id, "reconciled after server restart")
                .await?;
            closed += 1;
        }
        Ok(closed)
    }

    pub async fn run_udp(self: Arc<Self>, socket: UdpSocket) -> Result<(), KnockError> {
        let mut buffer = [0_u8; 2048];
        loop {
            let (length, peer) = socket.recv_from(&mut buffer).await?;
            let SocketAddr::V4(peer) = peer else {
                warn!("ignored IPv6 knock");
                continue;
            };
            if let Err(error) = self
                .process_packet(&buffer[..length], *peer.ip(), unix_time())
                .await
            {
                warn!(%error, source = %peer.ip(), "knock processing failed");
            }
        }
    }
}

pub fn make_knock_packet(session_id: Uuid, encoded_secret: &str) -> Result<Vec<u8>, KnockError> {
    let secret = URL_SAFE_NO_PAD
        .decode(encoded_secret)
        .map_err(|_| KnockError::InvalidPacket)?;
    if secret.len() != 32 {
        return Err(KnockError::InvalidPacket);
    }
    let mut packet = Vec::with_capacity(KNOCK_PACKET_BYTES);
    packet.extend_from_slice(KNOCK_MAGIC);
    packet.extend_from_slice(session_id.as_bytes());
    packet.extend_from_slice(&secret);
    Ok(packet)
}

fn parse_packet(packet: &[u8]) -> Result<(Uuid, [u8; 32]), KnockError> {
    if packet.len() != KNOCK_PACKET_BYTES || &packet[..4] != KNOCK_MAGIC {
        return Err(KnockError::InvalidPacket);
    }
    let session_id = Uuid::from_slice(&packet[4..20]).map_err(|_| KnockError::InvalidPacket)?;
    let secret: [u8; 32] = packet[20..52]
        .try_into()
        .map_err(|_| KnockError::InvalidPacket)?;
    Ok((session_id, secret))
}

#[must_use]
pub fn unix_time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knock_packet_is_fixed_and_round_trips() {
        let session_id = Uuid::new_v4();
        let token = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        let packet = make_knock_packet(session_id, &token).expect("packet");
        assert_eq!(packet.len(), KNOCK_PACKET_BYTES);
        let (decoded_id, secret) = parse_packet(&packet).expect("parse");
        assert_eq!(decoded_id, session_id);
        assert_eq!(secret, [7_u8; 32]);
    }
}
