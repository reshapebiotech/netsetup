use hickory_server::{
    authority::MessageResponseBuilder,
    proto::{
        op::{Header, ResponseCode},
        rr::{RData, Record, RecordType, rdata::A},
    },
    server::{Request, RequestHandler, ResponseHandler},
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
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> hickory_server::server::ResponseInfo {
        let mut header = Header::response_from_request(request.header());
        header.set_authoritative(true);

        let query = match request.queries().first() {
            Some(q) => q,
            None => {
                header.set_response_code(ResponseCode::FormErr);
                return response_handle
                    .send_response(
                        MessageResponseBuilder::from_message_request(request)
                            .build_no_records(header),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        let mut h = Header::new();
                        h.set_response_code(ResponseCode::ServFail);
                        h.into()
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
            header.set_response_code(ResponseCode::NXDomain);
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
                let mut header = Header::new();
                header.set_response_code(ResponseCode::ServFail);
                header.into()
            }
        }
    }
}
