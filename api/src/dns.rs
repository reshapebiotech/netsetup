use hickory_server::{
    net::runtime::Time,
    proto::{
        op::{Header, HeaderCounts, Metadata, ResponseCode},
        rr::{RData, Record, RecordType, rdata::A},
    },
    server::{Request, RequestHandler, ResponseHandler},
    zone_handler::MessageResponseBuilder,
};
use std::net::Ipv4Addr;
use tracing::{info, warn};

#[derive(Clone)]
pub struct CaptivePortalDns {
    pub captive_ip: Ipv4Addr,
}

impl CaptivePortalDns {
    pub fn new(captive_ip: Ipv4Addr) -> Self {
        Self { captive_ip }
    }
}

#[async_trait::async_trait]
impl RequestHandler for CaptivePortalDns {
    async fn handle_request<R: ResponseHandler, T: Time>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> hickory_server::server::ResponseInfo {
        let mut header = Metadata::response_from_request(&request.metadata);
        header.authoritative = true;

        let query = match request.queries.queries().first() {
            Some(q) => q,
            None => {
                header.response_code = ResponseCode::FormErr;
                return response_handle
                    .send_response(
                        MessageResponseBuilder::from_message_request(request)
                            .build_no_records(header),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        header.response_code = ResponseCode::ServFail;
                        Header {
                            metadata: header,
                            counts: HeaderCounts::default(),
                        }
                        .into()
                    });
            }
        };

        let name = query.name();
        let record_type = query.query_type();

        info!("DNS query: {} ({})", name, record_type);

        let builder = MessageResponseBuilder::from_message_request(request);
        let mut records = vec![];

        if record_type == RecordType::A {
            let record = Record::from_rdata(name.clone().into(), 60, RData::A(A(self.captive_ip)));
            records.push(record);
        } else {
            // NXDOMAIN for anything else to force resolution to fail or retry as A
            header.response_code = ResponseCode::NXDomain;
        }

        let response = builder.build(
            header,
            &records,
            &[], // namervers
            &[], // additionals
            &[], // additionals
        );

        match response_handle.send_response(response).await {
            Ok(info) => info,
            Err(e) => {
                warn!("Failed to send DNS response: {}", e);
                header.response_code = ResponseCode::ServFail;
                Header {
                    metadata: header,
                    counts: HeaderCounts::default(),
                }
                .into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_server::{
        Server,
        proto::{
            op::{Message, MessageType, OpCode, Query},
            rr::Name,
        },
    };
    use std::time::Duration;
    use tokio::{net::UdpSocket, time::timeout};

    #[tokio::test]
    async fn captive_dns_answers_ipv4_and_rejects_other_queries() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let captive_ip = Ipv4Addr::new(10, 42, 0, 1);
        let mut server = Server::new(CaptivePortalDns::new(captive_ip));
        server.register_socket(socket);
        let task = tokio::spawn(async move { server.block_until_done().await });
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.connect(address).await.unwrap();

        for (record_type, expected) in [
            (Some(RecordType::A), ResponseCode::NoError),
            (Some(RecordType::AAAA), ResponseCode::NXDomain),
            (None, ResponseCode::FormErr),
        ] {
            let mut query = Message::new(42, MessageType::Query, OpCode::Query);
            if let Some(record_type) = record_type {
                query.add_query(Query::query(
                    Name::from_ascii("example.com.").unwrap(),
                    record_type,
                ));
            }
            client.send(&query.to_vec().unwrap()).await.unwrap();
            let mut bytes = [0; 4096];
            let length = timeout(Duration::from_secs(2), client.recv(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            let response = Message::from_vec(&bytes[..length]).unwrap();
            assert_eq!(response.id, query.id);
            assert_eq!(response.message_type, MessageType::Response);
            assert_eq!(response.response_code, expected);
            if record_type == Some(RecordType::A) {
                assert!(response.authoritative);
                assert_eq!(response.answers.len(), 1);
                assert_eq!(response.answers[0].data, RData::A(A(captive_ip)));
            } else {
                assert!(response.answers.is_empty());
            }
        }
        task.abort();
    }
}
