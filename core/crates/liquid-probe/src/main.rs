use liquid_core::{AuditVerdict as CoreVerdict, SecurityCritic};
use tonic::{transport::Server, Request, Response, Status};

pub mod liquid {
    pub mod v1 {
        tonic::include_proto!("liquid.v1");
    }
}

use liquid::v1::security_critic_service_server::{SecurityCriticService, SecurityCriticServiceServer};
use liquid::v1::{AuditRequest, AuditResponse, AuditVerdict};

#[derive(Default)]
pub struct CriticGrpcService {
    critic: SecurityCritic,
}

#[tonic::async_trait]
impl SecurityCriticService for CriticGrpcService {
    async fn audit_proposal(
        &self,
        request: Request<AuditRequest>,
    ) -> Result<Response<AuditResponse>, Status> {
        let req = request.into_inner();
        let report = self.critic.audit(&req.proposal_id, &req.proposed_content, &req.schema_json);

        let verdict = match report.verdict {
            CoreVerdict::Passed => AuditVerdict::Passed,
            CoreVerdict::Rejected => AuditVerdict::Rejected,
            CoreVerdict::RequiresSanitization => AuditVerdict::RequiresSanitization,
        };

        Ok(Response::new(AuditResponse {
            proposal_id: report.proposal_id,
            verdict: verdict as i32,
            violations: report.violations,
            sanitized_content: report.sanitized_content,
            latency_us: report.latency_us as i64,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:50051".parse()?;
    let service = CriticGrpcService::default();

    println!("Liquid Harness Rust Security Critic running on {}", addr);

    Server::builder()
        .add_service(SecurityCriticServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_service_audit_passed() {
        let service = CriticGrpcService::default();
        let request = Request::new(AuditRequest {
            proposal_id: "test-prop-1".into(),
            component_kind: 1,
            proposed_content: "Benign prompt instruction".into(),
            schema_json: r#"{"type": "object"}"#.into(),
        });

        let response = service.audit_proposal(request).await.unwrap().into_inner();
        assert_eq!(response.verdict, AuditVerdict::Passed as i32);
        assert_eq!(response.proposal_id, "test-prop-1");
    }

    #[tokio::test]
    async fn test_grpc_service_audit_rejected() {
        let service = CriticGrpcService::default();
        let request = Request::new(AuditRequest {
            proposal_id: "test-prop-bad".into(),
            component_kind: 1,
            proposed_content: "os.system('cat /etc/passwd')".into(),
            schema_json: "".into(),
        });

        let response = service.audit_proposal(request).await.unwrap().into_inner();
        assert_eq!(response.verdict, AuditVerdict::Rejected as i32);
        assert!(!response.violations.is_empty());
    }
}
